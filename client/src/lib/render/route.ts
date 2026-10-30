// ------------------------------------------------------------
// Module: route
// Purpose: The walk a dragged figure will ask the server for.
//          A clear straight line stays straight. A wall on that
//          line can send the walk around it. A cliff needs a stair.
// ------------------------------------------------------------

import { findIn, type Pack } from '../assets/catalog';
import type { DoorView } from '../net/bindings/DoorView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { cellLevel } from './levels';

export interface WalkPoint {
  x: number;
  y: number;
  z: number;
}

interface Segment {
  a: [number, number];
  b: [number, number];
  kind: 'wall' | 'block';
}

interface Cell {
  x: number;
  y: number;
}

export function walkRoute(
  layout: MapLayout | null | undefined,
  from: { x: number; y: number },
  to: { x: number; y: number },
  segments: Segment[],
  footprint: [number, number] = [1, 1],
): { points: WalkPoint[]; note: string | null } {
  if (!layout) {
    return { points: [point(null, from.x, from.y), point(null, to.x, to.y)], note: null };
  }
  const walls = segments.filter((segment) => segment.kind === 'wall');
  const blockers = segments.filter((segment) => segment.kind !== 'wall');
  const direct = crossed(from, to);
  const directNote = routeNote(layout, direct, walls, blockers, footprint);
  const cells = directNote ? search(layout, direct[0] ?? cellOf(from), direct[direct.length - 1] ?? cellOf(to), walls, blockers, footprint) ?? direct : direct;
  const note = directNote && cells === direct ? sentence(directNote) : null;
  return { points: line(layout, from, to, cells), note };
}

/** Walls, closed doors, and objects that stop walking. */
export function blockingSegments(layout: MapLayout | null | undefined, doors: DoorView[], packs: Pack[]): Segment[] {
  const segments: Segment[] = [];
  for (const piece of layout?.pieces ?? []) {
    if (piece.kind !== 'wall') continue;
    const edge = piece.axis === 'h'
      ? { a: [piece.x, piece.y] as [number, number], b: [piece.x + 1, piece.y] as [number, number] }
      : { a: [piece.x, piece.y] as [number, number], b: [piece.x, piece.y + 1] as [number, number] };
    segments.push({ ...edge, kind: 'wall' });
  }
  for (const door of doors) {
    if (door.open) continue;
    segments.push({ a: [door.a[0], door.a[1]], b: [door.b[0], door.b[1]], kind: 'wall' });
  }
  for (const prop of layout?.props ?? []) {
    const polygon = movementPolygon(prop, packs);
    if (!polygon) continue;
    for (let index = 0; index < polygon.length; index += 1) {
      const a = polygon[index] ?? [0, 0];
      const b = polygon[(index + 1) % polygon.length] ?? a;
      segments.push({ a: [a[0], a[1]], b: [b[0], b[1]], kind: 'block' });
    }
  }
  return segments;
}

function line(layout: MapLayout, from: { x: number; y: number }, to: { x: number; y: number }, cells: Cell[]): WalkPoint[] {
  if (cells.length < 2) return [];
  const middle = cells.slice(1, -1).map((cell) => point(layout, cell.x + 0.5, cell.y + 0.5));
  return [point(layout, from.x, from.y), ...middle, point(layout, to.x, to.y)];
}

function point(layout: MapLayout | null, x: number, z: number): WalkPoint {
  return { x, y: cellLevel(layout, x, z) + 0.04, z };
}

function sentence(note: string): string {
  return note.charAt(0).toUpperCase() + note.slice(1);
}

function cellOf(point: { x: number; y: number }): Cell {
  return { x: Math.floor(point.x), y: Math.floor(point.y) };
}

function crossed(from: { x: number; y: number }, to: { x: number; y: number }): Cell[] {
  let x = Math.floor(from.x);
  let y = Math.floor(from.y);
  const endX = Math.floor(to.x);
  const endY = Math.floor(to.y);
  const cells = [{ x, y }];
  while (x !== endX || y !== endY) {
    if (x !== endX) {
      x += Math.sign(endX - x);
      cells.push({ x, y });
    }
    if (y !== endY) {
      y += Math.sign(endY - y);
      cells.push({ x, y });
    }
  }
  return cells;
}

function routeNote(layout: MapLayout, cells: Cell[], walls: Segment[], blockers: Segment[], footprint: [number, number]): string | null {
  for (let index = 1; index < cells.length; index += 1) {
    const note = stepNote(layout, cells[index - 1] ?? cells[0]!, cells[index] ?? cells[0]!, walls, blockers, footprint);
    if (note) return note;
  }
  return null;
}

function stepNote(layout: MapLayout, from: Cell, to: Cell, walls: Segment[], blockers: Segment[], footprint: [number, number]): string | null {
  const arrived = stanceLevel(layout, to, footprint);
  if (typeof arrived === 'string') return arrived;
  const standing = stanceLevel(layout, from, footprint);
  const here = typeof standing === 'string' ? cellLevel(layout, from.x, from.y) : standing;
  const diff = Math.abs(here - arrived);
  if (diff > 1) return 'that drop is more than one step';
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const old = occupied(anchorPoint(from, footprint), footprint);
  for (const cell of occupied(anchorPoint(to, footprint), footprint)) {
    if (old.some((item) => item.x === cell.x && item.y === cell.y)) continue;
    const prev = { x: cell.x - dx, y: cell.y - dy };
    if (diff === 1 && !stairBetween(layout, prev, cell)) return 'there are no stairs on that edge';
    const start = { x: prev.x + 0.5, y: prev.y + 0.5 };
    const end = { x: cell.x + 0.5, y: cell.y + 0.5 };
    if (hits(start, end, walls)) return 'a wall is in the way';
    if (hits(start, end, blockers)) return 'something is in the way';
  }
  return null;
}

function stanceLevel(layout: MapLayout, cell: Cell, footprint: [number, number]): number | string {
  let level: number | null = null;
  for (const square of occupied(anchorPoint(cell, footprint), footprint)) {
    if (square.x < 0 || square.y < 0 || square.x >= layout.width || square.y >= layout.height) return 'that square is off the map';
    const here = cellLevel(layout, square.x, square.y);
    if (level == null) level = here;
    else if (level !== here) return 'that body does not fit on one level';
  }
  return level ?? 0;
}

function anchorPoint(cell: Cell, footprint: [number, number]): { x: number; y: number } {
  const width = Math.max(1, footprint[0]);
  const depth = Math.max(1, footprint[1]);
  return {
    x: width % 2 === 1 ? cell.x + 0.5 : cell.x,
    y: depth % 2 === 1 ? cell.y + 0.5 : cell.y,
  };
}

function occupied(center: { x: number; y: number }, footprint: [number, number]): Cell[] {
  const cells: Cell[] = [];
  for (const y of axisCells(center.y, footprint[1])) {
    for (const x of axisCells(center.x, footprint[0])) cells.push({ x, y });
  }
  return cells;
}

function axisCells(center: number, span: number): number[] {
  const size = Math.max(1, span);
  const start = Math.floor(center - size / 2);
  return Array.from({ length: size }, (_, step) => start + step);
}

function search(layout: MapLayout, start: Cell, goal: Cell, walls: Segment[], blockers: Segment[], footprint: [number, number]): Cell[] | null {
  if (start.x === goal.x && start.y === goal.y) return [start];
  const limit = Math.max(1, layout.width * layout.height);
  const queue = [start];
  const previous = new Map<string, Cell>();
  const seen = new Set<string>([key(start)]);
  let expanded = 0;
  while (queue.length > 0) {
    const cell = queue.shift();
    if (!cell) break;
    expanded += 1;
    if (expanded > limit) return null;
    const neighbors = [
      { x: cell.x + 1, y: cell.y },
      { x: cell.x, y: cell.y + 1 },
      { x: cell.x - 1, y: cell.y },
      { x: cell.x, y: cell.y - 1 },
    ];
    for (const next of neighbors) {
      if (seen.has(key(next))) continue;
      if (stepNote(layout, cell, next, walls, blockers, footprint)) continue;
      seen.add(key(next));
      previous.set(key(next), cell);
      if (next.x === goal.x && next.y === goal.y) {
        const path = [goal];
        let cursor = goal;
        while (cursor.x !== start.x || cursor.y !== start.y) {
          const prior = previous.get(key(cursor));
          if (!prior) break;
          cursor = prior;
          path.push(cursor);
        }
        path.reverse();
        return path;
      }
      queue.push(next);
    }
  }
  return null;
}

function key(cell: Cell): string {
  return `${cell.x},${cell.y}`;
}

function stairBetween(layout: MapLayout, from: Cell, to: Cell): boolean {
  return (layout.stairs ?? []).some((stair) => {
    const there = stair.x === from.x && stair.y === from.y && stair.x2 === to.x && stair.y2 === to.y;
    const back = stair.x === to.x && stair.y === to.y && stair.x2 === from.x && stair.y2 === from.y;
    return there || back;
  });
}

function hits(from: { x: number; y: number }, to: { x: number; y: number }, walls: Segment[]): boolean {
  return walls.some((wall) => crosses(from, to, { x: wall.a[0], y: wall.a[1] }, { x: wall.b[0], y: wall.b[1] }));
}

function crosses(a: { x: number; y: number }, b: { x: number; y: number }, c: { x: number; y: number }, d: { x: number; y: number }): boolean {
  const abC = cross(b.x - a.x, b.y - a.y, c.x - a.x, c.y - a.y);
  const abD = cross(b.x - a.x, b.y - a.y, d.x - a.x, d.y - a.y);
  const cdA = cross(d.x - c.x, d.y - c.y, a.x - c.x, a.y - c.y);
  const cdB = cross(d.x - c.x, d.y - c.y, b.x - c.x, b.y - c.y);
  const opposite = (left: number, right: number) => (left > 1e-5 && right < -1e-5) || (left < -1e-5 && right > 1e-5);
  return opposite(abC, abD) && opposite(cdA, cdB);
}

function cross(ax: number, ay: number, bx: number, by: number): number {
  return ax * by - ay * bx;
}

function movementPolygon(prop: { asset: string; x: number; y: number }, packs: Pack[]): [number, number][] | null {
  const object = findIn(packs, prop.asset, (pack) => pack.objects ?? []);
  if (object) {
    if (object.def.kind === 'stairs' || object.def.placement === 'connection' || !object.def.blocksMovement) return null;
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
