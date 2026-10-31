// Cached collision grids and deterministic A* shared in behavior with server navigation.
import { findIn, type Pack } from '../assets/catalog';
import type { DoorView } from '../net/bindings/DoorView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { decodeCells } from '../editor/cells';
export interface WalkPoint {
  x: number;
  y: number;
  z: number;
}
export interface Segment {
  a: [number, number];
  b: [number, number];
  kind: 'wall' | 'block';
}
export type RouteLayout = Pick<MapLayout, 'width' | 'height' | 'levels' | 'stairs'>;
type Cell = [number, number];
type Walk = { points: WalkPoint[]; note: string | null };
const EPS = 0.0001,
  PROP = 1,
  WALL = 2;
const OFF = 'that square is off the map',
  BLOCK = 'something is in the way',
  WALL_NOTE = 'a wall is in the way',
  SPLIT = 'that body does not fit on one level',
  CLIFF = 'that drop is more than one step',
  STAIRS = 'there are no stairs on that edge';
const grids = new WeakMap<
  RouteLayout,
  {
    levels: RouteLayout['levels'];
    stairs: RouteLayout['stairs'];
    width: number;
    height: number;
    segments: Segment[];
    grid: Grid;
    lastKey?: string;
    last?: Walk;
  }
>();
const segmentsCache = new WeakMap<
  MapLayout,
  { pieces: MapLayout['pieces']; props: MapLayout['props']; doors: string; packs: Pack[]; value: Segment[] }
>();
export function walkRoute(
  layout: RouteLayout | null | undefined,
  from: { x: number; y: number },
  to: { x: number; y: number },
  segments: Segment[],
  footprint: [number, number] = [1, 1],
): Walk {
  if (!layout)
    return {
      points: [
        { x: from.x, y: 0.04, z: from.y },
        { x: to.x, y: 0.04, z: to.y },
      ],
      note: null,
    };
  if (
    ![from.x, from.y, to.x, to.y].every(Number.isFinite) ||
    layout.width * layout.height > 250000 ||
    layout.width * layout.height < 1
  )
    return { points: [], note: 'Choose a square on the map' };
  let cache = grids.get(layout);
  if (
    !cache ||
    cache.levels !== layout.levels ||
    cache.stairs !== layout.stairs ||
    cache.width !== layout.width ||
    cache.height !== layout.height ||
    cache.segments !== segments
  ) {
    cache = {
      levels: layout.levels,
      stairs: layout.stairs,
      width: layout.width,
      height: layout.height,
      segments,
      grid: new Grid(layout, segments),
    };
    grids.set(layout, cache);
  }
  const key = `${from.x},${from.y}:${to.x},${to.y}:${footprint}`;
  if (cache.lastKey === key && cache.last) return cache.last;
  const grid = cache.grid,
    start: Cell = [Math.floor(from.x), Math.floor(from.y)],
    goal: Cell = [Math.floor(to.x), Math.floor(to.y)],
    search = new Search(grid, footprint);
  const goalNote = search.standing(goal);
  if (grid.index(start) < 0 || grid.index(goal) < 0) return { points: [], note: 'That square is off the map' };
  const direct = crossed(start, goal);
  let reason: string | null = typeof goalNote === 'string' ? goalNote : null;
  if (!reason)
    for (let i = 1; i < direct.length; i++) {
      reason = search.step(direct[i - 1]!, direct[i]!);
      if (reason) break;
    }
  const cells = typeof goalNote === 'string' ? null : search.route(start, goal);
  const point = (x: number, z: number): WalkPoint => ({ x, y: grid.level([Math.floor(x), Math.floor(z)]) + 0.04, z });
  const route = cells ?? direct;
  const points =
    route.length < 2
      ? []
      : [
          point(from.x, from.y),
          ...route
            .slice(1, -1)
            .map(([x, y]) => point(x + (footprint[0] % 2 ? 0.5 : 0), y + (footprint[1] % 2 ? 0.5 : 0))),
          point(to.x, to.y),
        ];
  const note = cells ? null : (reason ?? 'there is no walkable path to that square');
  const walk = { points, note: note ? note.charAt(0).toUpperCase() + note.slice(1) : null };
  cache.lastKey = key;
  cache.last = walk;
  return walk;
}
/** Immutable snapshots let pointer events reuse collision data instead of scanning every object. */
export function blockingSegments(layout: MapLayout | null | undefined, doors: DoorView[], packs: Pack[]): Segment[] {
  const doorKey = doors.map((d) => `${d.id}:${+d.open}`).join('|'),
    cached = layout && segmentsCache.get(layout);
  if (
    cached &&
    cached.pieces === layout.pieces &&
    cached.props === layout.props &&
    cached.doors === doorKey &&
    cached.packs === packs
  )
    return cached.value;
  const value: Segment[] = [],
    placed = new Set<string>(),
    doorStates = new Map(doors.map((d) => [d.id, d.open]));
  for (const p of layout?.pieces ?? []) {
    if (p.kind === 'door') {
      placed.add(p.id);
      if (doorStates.get(p.id) ?? p.open) continue;
    }
    value.push({ a: [p.x, p.y], b: p.axis === 'h' ? [p.x + 1, p.y] : [p.x, p.y + 1], kind: 'wall' });
  }
  for (const d of doors)
    if (!d.open && !placed.has(d.id)) value.push({ a: [d.a[0], d.a[1]], b: [d.b[0], d.b[1]], kind: 'wall' });
  for (const prop of layout?.props ?? []) {
    const polygon = movementPolygon(prop, packs);
    if (!polygon) continue;
    for (let i = 0; i < polygon.length; i++)
      value.push({ a: polygon[i]!, b: polygon[(i + 1) % polygon.length]!, kind: 'block' });
  }
  if (layout) segmentsCache.set(layout, { pieces: layout.pieces, props: layout.props, doors: doorKey, packs, value });
  return value;
}
class Grid {
  readonly width: number;
  readonly height: number;
  readonly levels: Uint8Array;
  readonly blocked: Uint8Array;
  readonly edges: Uint8Array;
  readonly stairs = new Set<number>();
  readonly searches = new Map<string, { stance: Int16Array; steps: Uint8Array }>();
  constructor(layout: RouteLayout, segments: Segment[]) {
    this.width = layout.width;
    this.height = layout.height;
    const count = this.width * this.height;
    this.blocked = new Uint8Array(count);
    this.edges = new Uint8Array(count);
    try {
      this.levels = layout.levels ? decodeCells(layout.levels) : new Uint8Array(count);
    } catch {
      this.levels = new Uint8Array(count);
    }
    let polygon: Cell[] = [];
    for (const s of segments) {
      if (s.kind === 'wall') {
        this.wall(s);
        continue;
      }
      this.paint(s, PROP);
      if (polygon.length && !same(polygon[polygon.length - 1]!, s.a)) polygon = [];
      if (!polygon.length) polygon.push(s.a);
      polygon.push(s.b);
      if (polygon.length >= 4 && same(polygon[0]!, s.b)) {
        this.fill(polygon);
        polygon = [];
      }
    }
    for (const s of layout.stairs ?? []) {
      const a = this.index([s.x, s.y]),
        b = this.index([s.x2, s.y2]);
      if (a >= 0 && b >= 0) this.stairs.add(this.pair(a, b));
    }
  }
  index([x, y]: Cell) {
    return x < 0 || y < 0 || x >= this.width || y >= this.height ? -1 : y * this.width + x;
  }
  cell(i: number): Cell {
    return [i % this.width, Math.floor(i / this.width)];
  }
  level(cell: Cell) {
    return this.levels[this.index(cell)] ?? 0;
  }
  pair(a: number, b: number) {
    return Math.min(a, b) * this.blocked.length + Math.max(a, b);
  }
  edge(a: Cell, b: Cell) {
    const dx = b[0] - a[0],
      dy = b[1] - a[1],
      bit = dx === 1 ? 1 : dy === 1 ? 2 : dx === -1 ? 4 : dy === -1 ? 8 : 0;
    return ((this.edges[this.index(a)] ?? 0) & bit) !== 0;
  }
  wall(s: Segment) {
    const [ax, ay] = s.a,
      [bx, by] = s.b;
    if (Math.abs(ax - bx) < EPS && Math.abs(ax - Math.round(ax)) < EPS) {
      const x = Math.round(ax);
      for (
        let y = Math.max(0, Math.floor(Math.min(ay, by)));
        y < Math.min(this.height, Math.ceil(Math.max(ay, by)));
        y++
      ) {
        const l = this.index([x - 1, y]),
          r = this.index([x, y]);
        if (l >= 0) this.edges[l]! |= 1;
        if (r >= 0) this.edges[r]! |= 4;
      }
    } else if (Math.abs(ay - by) < EPS && Math.abs(ay - Math.round(ay)) < EPS) {
      const y = Math.round(ay);
      for (
        let x = Math.max(0, Math.floor(Math.min(ax, bx)));
        x < Math.min(this.width, Math.ceil(Math.max(ax, bx)));
        x++
      ) {
        const t = this.index([x, y - 1]),
          b = this.index([x, y]);
        if (t >= 0) this.edges[t]! |= 2;
        if (b >= 0) this.edges[b]! |= 8;
      }
    } else this.paint(s, WALL);
  }
  paint(s: Segment, bit: number) {
    const [ax, ay] = s.a,
      [bx, by] = s.b;
    for (
      let y = Math.max(0, Math.floor(Math.min(ay, by)) - 1);
      y <= Math.min(this.height - 1, Math.floor(Math.max(ay, by)));
      y++
    ) {
      // Clip to a row so long diagonal barriers don't scan a whole bounding rectangle.
      let left = Math.min(ax, bx),
        right = Math.max(ax, bx);
      if (Math.abs(by - ay) >= EPS) {
        const p = (y + EPS - ay) / (by - ay),
          q = (y + 1 - EPS - ay) / (by - ay);
        const lo = Math.max(0, Math.min(p, q)),
          hi = Math.min(1, Math.max(p, q));
        if (lo > hi) continue;
        const x0 = ax + (bx - ax) * lo,
          x1 = ax + (bx - ax) * hi;
        left = Math.min(x0, x1);
        right = Math.max(x0, x1);
      }
      for (let x = Math.max(0, Math.floor(left) - 1); x <= Math.min(this.width - 1, Math.floor(right)); x++) {
        if (intersectsCell(s.a, s.b, x, y)) this.blocked[y * this.width + x]! |= bit;
      }
    }
  }
  fill(poly: Cell[]) {
    const min = Math.floor(Math.min(...poly.map((p) => p[1]))),
      max = Math.ceil(Math.max(...poly.map((p) => p[1])));
    for (let y = Math.max(0, min); y < Math.min(this.height, max); y++) {
      const scan = y + 0.5,
        cuts: number[] = [];
      for (let i = 1; i < poly.length; i++) {
        const a = poly[i - 1]!,
          b = poly[i]!;
        if (a[1] > scan !== b[1] > scan) cuts.push(a[0] + ((scan - a[1]) * (b[0] - a[0])) / (b[1] - a[1]));
      }
      cuts.sort((a, b) => a - b);
      for (let i = 1; i < cuts.length; i += 2)
        for (
          let x = Math.max(0, Math.ceil(cuts[i - 1]! - 0.5 + EPS));
          x <= Math.min(this.width - 1, Math.floor(cuts[i]! - 0.5 - EPS));
          x++
        )
          this.blocked[y * this.width + x]! |= PROP;
    }
  }
}
function same(a: Cell, b: Cell) {
  return Math.abs(a[0] - b[0]) < EPS && Math.abs(a[1] - b[1]) < EPS;
}
function intersectsCell(a: Cell, b: Cell, x: number, y: number) {
  let lo = 0,
    hi = 1;
  const axes: [number, number, number, number][] = [
    [a[0], b[0] - a[0], x + EPS, x + 1 - EPS],
    [a[1], b[1] - a[1], y + EPS, y + 1 - EPS],
  ];
  for (const [start, delta, min, max] of axes) {
    if (Math.abs(delta) < EPS) {
      if (start < min || start > max) return false;
    } else {
      const p = (min - start) / delta,
        q = (max - start) / delta;
      lo = Math.max(lo, Math.min(p, q));
      hi = Math.min(hi, Math.max(p, q));
      if (lo > hi) return false;
    }
  }
  return true;
}
class Search {
  readonly stance: Int16Array;
  readonly steps: Uint8Array;
  readonly single: boolean;
  constructor(
    readonly grid: Grid,
    readonly footprint: [number, number],
  ) {
    this.single = footprint[0] === 1 && footprint[1] === 1;
    const key = footprint.join(',');
    let cached = grid.searches.get(key);
    if (!cached) {
      if (grid.searches.size >= 4) grid.searches.delete(grid.searches.keys().next().value!);
      const stance = new Int16Array(grid.blocked.length);
      stance.fill(-1);
      const steps = new Uint8Array(grid.blocked.length * 8);
      steps.fill(255);
      cached = { stance, steps };
      grid.searches.set(key, cached);
    }
    this.stance = cached.stance;
    this.steps = cached.steps;
  }
  body([cx, cy]: Cell) {
    const w = Math.max(1, this.footprint[0]),
      h = Math.max(1, this.footprint[1]);
    return { x: cx - Math.floor(w / 2), y: cy - Math.floor(h / 2), w, h };
  }
  standing(cell: Cell): number | string {
    const index = this.grid.index(cell);
    if (index < 0) return OFF;
    const saved = this.stance[index]!;
    if (saved >= 0) return decode(saved);
    if (this.single) {
      const blocked = this.grid.blocked[index]!;
      const result = blocked & PROP ? BLOCK : blocked & WALL ? WALL_NOTE : this.grid.levels[index]!;
      this.stance[index] = encode(result);
      return result;
    }
    const { x, y, w, h } = this.body(cell);
    let result: number | string = this.grid.level(cell);
    body: for (let cy = y; cy < y + h; cy++)
      for (let cx = x; cx < x + w; cx++) {
        const i = this.grid.index([cx, cy]);
        if (i < 0) {
          result = OFF;
          break body;
        }
        if (this.grid.blocked[i]! & PROP) {
          result = BLOCK;
          break body;
        }
        if (
          this.grid.blocked[i]! & WALL ||
          (cx > x && this.grid.edge([cx - 1, cy], [cx, cy])) ||
          (cy > y && this.grid.edge([cx, cy - 1], [cx, cy]))
        ) {
          result = WALL_NOTE;
          break body;
        }
        if (this.grid.levels[i] !== this.grid.level(cell)) {
          result = SPLIT;
          break body;
        }
      }
    this.stance[index] = encode(result);
    return result;
  }
  cardinal(from: Cell, to: Cell): string | null {
    const there = this.standing(to);
    if (typeof there === 'string') return there;
    const diff = Math.abs(this.grid.level(from) - there);
    if (diff > 1) return CLIFF;
    if (this.single) {
      if (this.grid.edge(from, to)) return WALL_NOTE;
      if (diff === 1 && !this.grid.stairs.has(this.grid.pair(this.grid.index(from), this.grid.index(to))))
        return STAIRS;
      return null;
    }
    const { x, y, w, h } = this.body(to),
      dx = to[0] - from[0],
      dy = to[1] - from[1];
    for (let cy = y; cy < y + h; cy++)
      for (let cx = x; cx < x + w; cx++) {
        if (
          (dx === 1 && cx !== x + w - 1) ||
          (dx === -1 && cx !== x) ||
          (dy === 1 && cy !== y + h - 1) ||
          (dy === -1 && cy !== y)
        )
          continue;
        const prev: Cell = [cx - dx, cy - dy],
          next: Cell = [cx, cy];
        if (this.grid.edge(prev, next)) return WALL_NOTE;
        if (diff === 1) {
          const a = this.grid.index(prev),
            b = this.grid.index(next);
          if (a < 0 || b < 0) return OFF;
          if (!this.grid.stairs.has(this.grid.pair(a, b))) return STAIRS;
        }
      }
    return null;
  }
  step(from: Cell, to: Cell): string | null {
    if (from[0] === to[0] || from[1] === to[1]) return this.cardinal(from, to);
    const a: Cell = [to[0], from[1]],
      b: Cell = [from[0], to[1]];
    if ([a, b, to].some((p) => this.grid.level(p) !== this.grid.level(from))) return STAIRS;
    if (this.single) {
      for (const cell of [a, b, to]) {
        const stance = this.standing(cell);
        if (typeof stance === 'string') return stance;
      }
      return this.grid.edge(from, a) || this.grid.edge(from, b) || this.grid.edge(a, to) || this.grid.edge(b, to)
        ? WALL_NOTE
        : null;
    }
    return this.cardinal(from, a) ?? this.cardinal(from, b) ?? this.cardinal(a, to) ?? this.cardinal(b, to);
  }
  route(start: Cell, goal: Cell): Cell[] | null {
    const startI = this.grid.index(start),
      goalI = this.grid.index(goal);
    if (startI < 0 || goalI < 0) return null;
    if (startI === goalI) return [start];
    const distance = new Uint32Array(this.grid.blocked.length);
    distance.fill(0xffffffff);
    const previous = new Int32Array(this.grid.blocked.length);
    previous.fill(-1);
    const closed = new Uint8Array(this.grid.blocked.length),
      heap = new Heap();
    let serial = 0;
    const h = heuristic(start, goal);
    distance[startI] = 0;
    heap.push([h, h, serial, startI]);
    while (heap.length) {
      const index = heap.pop()![3];
      if (closed[index]) continue;
      closed[index] = 1;
      const from = this.grid.cell(index);
      if (index === goalI) {
        const path = [goal];
        let cursor = index;
        while (cursor !== startI) {
          cursor = previous[cursor]!;
          path.push(this.grid.cell(cursor));
        }
        return path.reverse();
      }
      for (let direction = 0; direction < DIRS.length; direction++) {
        const [dx, dy] = DIRS[direction]!;
        const next: Cell = [from[0] + dx, from[1] + dy],
          ni = this.grid.index(next);
        if (ni < 0 || closed[ni]) continue;
        const stepIndex = index * 8 + direction;
        if (this.steps[stepIndex] === 255) this.steps[stepIndex] = this.step(from, next) ? 1 : 0;
        if (this.steps[stepIndex]) continue;
        const cost = distance[index]! + (dx && dy ? 14 : 10);
        if (cost >= distance[ni]!) continue;
        distance[ni] = cost;
        previous[ni] = index;
        const h = heuristic(next, goal);
        heap.push([cost + h, h, ++serial, ni]);
      }
    }
    return null;
  }
}
const DIRS: Cell[] = [
  [1, 0],
  [0, 1],
  [-1, 0],
  [0, -1],
  [1, 1],
  [-1, 1],
  [-1, -1],
  [1, -1],
];
type Entry = [number, number, number, number];
const before = (a: Entry, b: Entry) =>
  a[0] < b[0] || (a[0] === b[0] && (a[1] < b[1] || (a[1] === b[1] && a[2] < b[2])));
class Heap {
  items: Entry[] = [];
  get length() {
    return this.items.length;
  }
  push(e: Entry) {
    let i = this.items.length;
    this.items.push(e);
    while (i > 0) {
      const p = (i - 1) >> 1;
      if (!before(e, this.items[p]!)) break;
      this.items[i] = this.items[p]!;
      i = p;
    }
    this.items[i] = e;
  }
  pop() {
    const first = this.items[0],
      last = this.items.pop();
    if (last && this.items.length) {
      let i = 0;
      while (i * 2 + 1 < this.items.length) {
        let c = i * 2 + 1;
        if (c + 1 < this.items.length && before(this.items[c + 1]!, this.items[c]!)) c++;
        if (!before(this.items[c]!, last)) break;
        this.items[i] = this.items[c]!;
        i = c;
      }
      this.items[i] = last;
    }
    return first;
  }
}
function heuristic(a: Cell, b: Cell) {
  const dx = Math.abs(a[0] - b[0]),
    dy = Math.abs(a[1] - b[1]);
  return 10 * Math.max(dx, dy) + 4 * Math.min(dx, dy);
}
function encode(r: number | string) {
  return typeof r === 'number' ? r : r === OFF ? 10 : r === BLOCK ? 11 : r === WALL_NOTE ? 12 : 13;
}
function decode(v: number): number | string {
  return v === 10 ? OFF : v === 11 ? BLOCK : v === 12 ? WALL_NOTE : v === 13 ? SPLIT : v;
}
function crossed(start: Cell, goal: Cell) {
  let [x, y] = start;
  const cells: Cell[] = [start];
  while (x !== goal[0] || y !== goal[1]) {
    if (x !== goal[0]) {
      x += Math.sign(goal[0] - x);
      cells.push([x, y]);
    }
    if (y !== goal[1]) {
      y += Math.sign(goal[1] - y);
      cells.push([x, y]);
    }
  }
  return cells;
}
function movementPolygon(prop: { asset: string; x: number; y: number }, packs: Pack[]): [number, number][] | null {
  const object = findIn(packs, prop.asset, (pack) => pack.objects ?? []);
  if (object) {
    if (object.def.kind === 'stairs' || object.def.placement === 'connection' || !object.def.blocksMovement)
      return null;
    const origin = object.def.originCells ?? [object.def.footprintCells[0] / 2, object.def.footprintCells[1] / 2];
    const polygon = object.def.collisionPolygon ?? [];
    if (polygon.length >= 3) return polygon.map(([x, y]) => [prop.x + origin[0] + x, prop.y + origin[1] + y]);
    const width = object.def.footprintCells[0];
    const depth = object.def.footprintCells[1];
    return [
      [prop.x, prop.y],
      [prop.x + width, prop.y],
      [prop.x + width, prop.y + depth],
      [prop.x, prop.y + depth],
    ];
  }
  const legacy = findIn(packs, prop.asset, (pack) => pack.props);
  if (!legacy || legacy.def.blocks !== 'move') return null;
  const [width, depth] = legacy.def.footprint;
  return [
    [prop.x, prop.y],
    [prop.x + width, prop.y],
    [prop.x + width, prop.y + depth],
    [prop.x, prop.y + depth],
  ];
}
