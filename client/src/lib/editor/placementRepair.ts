// ------------------------------------------------------------
// Module: placementRepair
// Purpose: Name figures who would not fit on the draft before
//          it is put on the table. They are not moved.
// ------------------------------------------------------------

import { findIn, type Pack } from '../assets/catalog';
import { cellLevel } from '../render/levels';

export function propMirror(asset: string, packs: Pack[], requested: boolean): { mirror: boolean; note: string | null } {
  if (!requested) return { mirror: false, note: null };
  const object = findIn(packs, asset, (pack) => pack.objects ?? []);
  if (object) return { mirror: false, note: `${object.def.name} cannot be mirrored. Upright art needs its own picture.` };
  const legacy = findIn(packs, asset, (pack) => pack.props);
  if (legacy?.def.mirror) return { mirror: true, note: null };
  const name = legacy?.def.name ?? 'That object';
  return { mirror: false, note: `${name} cannot be mirrored. Upright art needs its own picture.` };
}

export function stairRepair(layout: {
  width: number;
  height: number;
  levels?: string | null;
  stairs?: { x: number; y: number; x2: number; y2: number }[] | null;
}): string | null {
  const bad = (layout.stairs ?? []).filter((stair) => !stairFits(layout, stair)).map((stair) => `(${stair.x}, ${stair.y}) to (${stair.x2}, ${stair.y2})`);
  if (bad.length === 0) return null;
  return `These stairs do not join neighboring squares one step apart: ${bad.join('; ')}.`;
}

function stairFits(
  layout: { width: number; height: number; levels?: string | null },
  stair: { x: number; y: number; x2: number; y2: number },
): boolean {
  if (stair.x >= layout.width || stair.y >= layout.height || stair.x2 >= layout.width || stair.y2 >= layout.height) return false;
  const dx = Math.abs(stair.x - stair.x2);
  const dy = Math.abs(stair.y - stair.y2);
  if (dx + dy !== 1) return false;
  return Math.abs(cellLevel(layout, stair.x, stair.y) - cellLevel(layout, stair.x2, stair.y2)) === 1;
}

export function objectLevels(
  layout: { width: number; height: number; levels?: string | null; props?: { asset: string; x: number; y: number }[] },
  packs: Pack[] = [],
): string | null {
  const names: string[] = [];
  for (const prop of layout.props ?? []) {
    const found = supportOf(prop.asset, packs);
    if (!found) continue;
    const problem = supportProblem(layout, prop.x, prop.y, found.span);
    if (problem) names.push(`${found.name} at ${trim(prop.x)}, ${trim(prop.y)} ${problem}`);
  }
  if (names.length === 0) return null;
  const move = names.length === 1 ? 'Move it' : 'Move each one';
  return `${names.join('; ')}. ${move} so the whole object sits on one level, then put the map on the table again.`;
}

function supportOf(asset: string, packs: Pack[]): { name: string; span: [number, number] } | null {
  const object = findIn(packs, asset, (pack) => pack.objects ?? []);
  if (object) {
    if (object.def.kind === 'stairs' || object.def.placement === 'connection') return null;
    return { name: object.def.name, span: object.def.footprintCells };
  }
  const legacy = findIn(packs, asset, (pack) => pack.props);
  if (!legacy) return null;
  return { name: legacy.def.name, span: legacy.def.footprint };
}

function supportProblem(
  layout: { width: number; height: number; levels?: string | null },
  x: number,
  y: number,
  span: [number, number],
): string | null {
  const width = Math.max(1, Math.floor(span[0]) || 1);
  const depth = Math.max(1, Math.floor(span[1]) || 1);
  const x0 = Math.floor(x);
  const y0 = Math.floor(y);
  let level: number | null = null;
  for (let dy = 0; dy < depth; dy += 1) {
    for (let dx = 0; dx < width; dx += 1) {
      const cx = x0 + dx;
      const cy = y0 + dy;
      if (cx < 0 || cy < 0 || cx >= layout.width || cy >= layout.height) return 'does not fit on the map';
      const here = cellLevel(layout, cx + 0.5, cy + 0.5);
      if (level == null) level = here;
      else if (level !== here) return 'stands on two heights';
    }
  }
  return null;
}

function trim(value: number): string {
  return Number.isInteger(value) ? String(value) : String(value);
}

export interface RepairToken {
  name: string;
  x: number;
  y: number;
  footprint?: [number, number] | null;
}

export function placementRepair(
  tokens: RepairToken[],
  layout: { width: number; height: number; levels?: string | null; props?: { asset: string; x: number; y: number }[] },
  packs: Pack[] = [],
): string | null {
  const outside = tokens
    .filter((token) => token.x < 0.5 || token.y < 0.5 || token.x > layout.width - 0.5 || token.y > layout.height - 0.5)
    .map((token) => token.name);
  const outsideNote = occupy(outside, 'outside the new map', 'Move them onto it, then put the map on the table again.');
  if (outsideNote) return outsideNote;
  const inside = tokens.filter((token) => standingIn(token, layout.props ?? [], packs)).map((token) => token.name);
  const insideNote = occupy(inside, 'standing in something on the new map', 'Move them clear of it, then put the map on the table again.');
  if (insideNote) return insideNote;
  const split = tokens.filter((token) => straddles(layout, token)).map((token) => token.name);
  return occupy(split, 'standing across two levels', 'Move them onto one level, then put the map on the table again.');
}

function standingIn(token: RepairToken, props: { asset: string; x: number; y: number }[], packs: Pack[]): boolean {
  return props.some((prop) => {
    const polygon = movementPolygon(prop, packs);
    return polygon != null && pointInPoly(token.x, token.y, polygon);
  });
}

function movementPolygon(prop: { asset: string; x: number; y: number }, packs: Pack[]): [number, number][] | null {
  const object = findIn(packs, prop.asset, (pack) => pack.objects ?? []);
  if (object) {
    if (object.def.kind === 'stairs' || object.def.placement === 'connection' || !object.def.blocksMovement) return null;
    const origin = object.def.originCells ?? [object.def.footprintCells[0] / 2, object.def.footprintCells[1] / 2];
    const rootX = prop.x + origin[0];
    const rootY = prop.y + origin[1];
    const polygon = object.def.collisionPolygon ?? [];
    if (polygon.length >= 3) return polygon.map(([x, y]) => [rootX + x, rootY + y]);
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

function pointInPoly(x: number, y: number, polygon: [number, number][]): boolean {
  if (polygon.length < 3) return false;
  let inside = false;
  let previous = polygon.length - 1;
  for (let index = 0; index < polygon.length; index += 1) {
    const a = polygon[index] ?? [0, 0];
    const b = polygon[previous] ?? [0, 0];
    const crosses = a[1] > y !== b[1] > y;
    if (crosses) {
      const xAt = ((b[0] - a[0]) * (y - a[1])) / (b[1] - a[1]) + a[0];
      if (x < xAt) inside = !inside;
    }
    previous = index;
  }
  return inside;
}

function straddles(layout: { width: number; height: number; levels?: string | null }, token: RepairToken): boolean {
  const [width, depth] = token.footprint ?? [1, 1];
  const xs = axis(token.x, width);
  const ys = axis(token.y, depth);
  let height: number | null = null;
  for (const y of ys) {
    for (const x of xs) {
      if (x < 0 || y < 0 || x >= layout.width || y >= layout.height) return false;
      const level = cellLevel(layout, x + 0.5, y + 0.5);
      if (height == null) height = level;
      else if (height !== level) return true;
    }
  }
  return false;
}

function axis(center: number, span: number): number[] {
  const size = Math.max(1, Math.floor(span) || 1);
  const start = Math.floor(center - size / 2);
  return Array.from({ length: size }, (_, step) => start + step);
}

function occupy(names: string[], whereTheyAre: string, repair: string): string | null {
  if (names.length === 0) return null;
  const list = names.length === 1 ? names[0] : names.length === 2 ? `${names[0]} and ${names[1]}` : `${names.slice(0, -1).join(', ')}, and ${names[names.length - 1]}`;
  const verb = names.length === 1 ? 'is' : 'are';
  return `${list} ${verb} ${whereTheyAre}. ${repair}`;
}
