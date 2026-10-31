// ------------------------------------------------------------
// Module: scatter
// Purpose: What a theme brush paints. Terrain first, then props
//          and decals, with a fixed seed so the same stroke
//          always lands the same way.
// ------------------------------------------------------------

import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { PlacedDecal } from '../net/bindings/PlacedDecal.ts';
import type { PlacedProp } from '../net/bindings/PlacedProp.ts';
import { decodeCells, encodeCells } from './cells';
import { clearVariantPins } from './variants';
import type { Cell } from './brush';

export interface Footprint {
  x: number;
  y: number;
  w: number;
  d: number;
}

interface Mark extends Footprint {
  asset: string;
}

export interface ScatterEntry {
  asset: string;
  density: number;
  spacing: number;
  kind: 'prop' | 'decal';
  footprint: [number, number];
  rotate: boolean;
}

export function footprintsOverlap(a: Footprint, b: Footprint) {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.d && b.y < a.y + a.d;
}

export function blocksDoor(foot: Footprint, layout: MapLayout) {
  return layout.pieces.some((piece) => {
    if (piece.kind !== 'door') return false;
    if (piece.axis === 'h') {
      return piece.y >= foot.y && piece.y <= foot.y + foot.d && piece.x < foot.x + foot.w && piece.x + 1 > foot.x;
    }
    return piece.x >= foot.x && piece.x <= foot.x + foot.w && piece.y < foot.y + foot.d && piece.y + 1 > foot.y;
  });
}

export function fillTheme(
  layout: MapLayout,
  brushed: Cell[],
  terrain: string,
  entries: ScatterEntry[],
  seed: number,
  footprintOf: (asset: string) => [number, number] = () => [1, 1],
): MapLayout {
  const next = structuredClone(layout);
  const index = ensureTerrain(next, terrain);
  const bytes = decodeCells(next.cells);
  for (const cell of brushed) {
    const offset = cell.y * next.width + cell.x;
    if (offset >= 0 && offset < bytes.length) bytes[offset] = index;
  }
  next.cells = encodeCells(bytes);

  const rng = mulberry32(seed);
  const brushedSet = new Set(brushed.map((cell) => cell.y * layout.width + cell.x));
  const levels = layout.levels ? decodeCells(layout.levels) : new Uint8Array(layout.width * layout.height);
  const occupied: Mark[] = next.props.map((prop) => {
    const [w, d] = footprintOf(prop.asset);
    return { x: prop.x, y: prop.y, w, d, asset: prop.asset };
  });
  let serial = 0;
  const ids = new Set(next.props.map((prop) => prop.id));
  for (const entry of entries) {
    const exact = (entry.density * brushed.length) / 10;
    let count = Math.floor(exact);
    if (rng() < exact - count) count += 1;
    // A second pass fills gaps instead of piling another population on the same biome.
    const existing = entry.kind === 'prop'
      ? next.props.filter((prop) => prop.asset === entry.asset && brushedSet.has(Math.floor(prop.y) * layout.width + Math.floor(prop.x))).length
      : next.decals.filter((decal) => decal.asset === entry.asset && brushedSet.has(Math.floor(decal.y) * layout.width + Math.floor(decal.x))).length;
    count = Math.max(0, count - existing);
    for (let placed = 0; placed < count; placed += 1) {
      const spot = place(next, brushed, brushedSet, levels, entry, occupied, rng);
      if (!spot) continue;
      occupied.push({ ...spot, asset: entry.asset });
      if (entry.kind === 'prop') {
        while (ids.has(`s${seed}-${serial}`)) serial += 1;
        ids.add(`s${seed}-${serial}`);
        const prop: PlacedProp = { id: `s${seed}-${serial}`, asset: entry.asset, x: spot.x, y: spot.y, mirror: false };
        serial += 1;
        next.props.push(prop);
      } else {
        const decal: PlacedDecal = {
          asset: entry.asset,
          x: spot.x + spot.w / 2,
          y: spot.y + spot.d / 2,
          scale: 1,
          angle: entry.rotate ? Math.floor(rng() * 360) : 0,
        };
        next.decals.push(decal);
      }
    }
  }
  return clearVariantPins(next, brushed);
}

function ensureTerrain(layout: MapLayout, terrain: string) {
  let index = layout.terrains.indexOf(terrain);
  if (index === -1) {
    layout.terrains.push(terrain);
    index = layout.terrains.length - 1;
  }
  return index;
}

function place(layout: MapLayout, brushed: Cell[], brushedSet: Set<number>, levels: Uint8Array, entry: ScatterEntry, occupied: Mark[], rng: () => number): Footprint | null {
  const [w, d] = entry.footprint;
  for (let attempt = 0; attempt < 16; attempt += 1) {
    const cell = brushed[Math.floor(rng() * brushed.length)];
    if (!cell) return null;
    const foot = { x: cell.x, y: cell.y, w, d };
    if (foot.x + w > layout.width || foot.y + d > layout.height) continue;
    if (blocksDoor(foot, layout)) continue;
    // Keep the complete footprint inside the painted biome and on one tier.
    let supported = true;
    const level = levels[cell.y * layout.width + cell.x] ?? 0;
    for (let y = foot.y; y < foot.y + d; y += 1) {
      for (let x = foot.x; x < foot.x + w; x += 1) {
        if (!brushedSet.has(y * layout.width + x) || (levels[y * layout.width + x] ?? 0) !== level) supported = false;
      }
    }
    if (!supported) continue;
    if (entry.kind === 'prop' && (layout.stairs ?? []).some((stair) => [ [stair.x, stair.y], [stair.x2, stair.y2] ].some(([x, y]) => x! >= foot.x && x! < foot.x + w && y! >= foot.y && y! < foot.y + d))) continue;
    if (entry.kind === 'prop' && layout.pieces.some((piece) => piece.axis === 'h'
      ? piece.y > foot.y && piece.y < foot.y + d && piece.x < foot.x + w && piece.x + 1 > foot.x
      : piece.x > foot.x && piece.x < foot.x + w && piece.y < foot.y + d && piece.y + 1 > foot.y)) continue;
    if (entry.kind === 'prop' && occupied.some((other) => footprintsOverlap(foot, other))) continue;
    if (entry.spacing > 0 && occupied.some((other) => other.asset === entry.asset && Math.hypot(other.x - foot.x, other.y - foot.y) < entry.spacing)) {
      continue;
    }
    return foot;
  }
  return null;
}

function mulberry32(seed: number) {
  let value = seed >>> 0;
  return () => {
    value = (value + 0x6d2b79f5) | 0;
    let t = Math.imul(value ^ (value >>> 15), 1 | value);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
