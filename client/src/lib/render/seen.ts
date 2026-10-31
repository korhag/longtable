// ------------------------------------------------------------
// Module: seen
// Purpose: Whether a ground point is inside the player's fog.
//          The GM, and a build session, see everything.
// ------------------------------------------------------------

import { bitAt, decodeBits } from '../fog/bits';
import type { FogView } from '../net/bindings/FogView.ts';

type Masks = { explored: string; visible?: string; exploredBits: Uint8Array; visibleBits: Uint8Array };
const masks = new WeakMap<FogView, Masks>();

/** Decode each received picture once, rather than once per prop or light. */
export function fogMasks(fog: FogView) {
  const previous = masks.get(fog);
  if (previous?.explored === fog.explored && previous.visible === fog.visible) return previous;
  const next = { explored: fog.explored, visible: fog.visible,
    exploredBits: decodeBits(fog.explored), visibleBits: decodeBits(fog.visible ?? '') };
  masks.set(fog, next);
  return next;
}

function maskAt(fog: FogView, bits: Uint8Array, x: number, y: number) {
  const scale = fog.cellsPerSquare || 4;
  const fx = Math.floor(x * scale);
  const fy = Math.floor(y * scale);
  // An x outside the map must not wrap into the following bitmap row.
  return fx >= 0 && fy >= 0 && fx < fog.width && fy < fog.height
    && bitAt(bits, fy * fog.width + fx);
}

/** Coordinates are world roots, already centered by the caller when needed. */
export function seen(fog: FogView | null, x: number, y: number) {
  if (!fog || fog.mode === 'gm') return true;
  return maskAt(fog, fogMasks(fog).exploredBits, x, y) || inSight({ x, y }, fog);
}

/** A player picture keeps a figure only where that seat can see, plus their own. */
export function inSight(
  token: { x: number; y: number; ownerId?: string | null },
  fog: FogView | null,
  seatId?: string | null,
): boolean {
  if (!fog || fog.mode === 'gm') return true;
  if (seatId && token.ownerId === seatId) return true;
  if (fog.visible) {
    return maskAt(fog, fogMasks(fog).visibleBits, token.x, token.y);
  }
  return fog.visiblePolygons.some((polygon) => pointInPoly(token.x, token.y, polygon));
}

/** Linear-light multiplier, matching the dark remembered ground under fog. */
export function sceneryBrightness(fog: FogView | null, x: number, y: number) {
  if (inSight({ x, y }, fog)) return 1;
  return seen(fog, x, y) ? 0.065 : 0;
}

/** A wall is visible from either adjacent side of its zero-width ground edge. */
export function edgeBrightness(fog: FogView | null, x: number, y: number, axis: string) {
  const step = 0.5 / (fog?.cellsPerSquare || 4);
  const dx = axis === 'v' ? step : 0;
  const dy = axis === 'h' ? step : 0;
  return Math.max(sceneryBrightness(fog, x - dx, y - dy), sceneryBrightness(fog, x + dx, y + dy));
}

/** A blocking trunk/rock cannot see its own center. Sample its exposed near faces. */
export function blockerBrightness(fog: FogView | null, x: number, y: number, footprint: [number, number], polygon?: [number, number][]) {
  let brightness = sceneryBrightness(fog, x, y);
  if (brightness === 1) return 1;
  const pad = 0.5 / (fog?.cellsPerSquare || 4);
  const minX = (polygon?.length ? Math.min(...polygon.map(point => point[0])) : -footprint[0] / 2) - pad;
  const maxX = (polygon?.length ? Math.max(...polygon.map(point => point[0])) : footprint[0] / 2) + pad;
  const minY = (polygon?.length ? Math.min(...polygon.map(point => point[1])) : -footprint[1] / 2) - pad;
  const maxY = (polygon?.length ? Math.max(...polygon.map(point => point[1])) : footprint[1] / 2) + pad;
  for (const dx of [minX, (minX + maxX) / 2, maxX]) for (const dy of [minY, (minY + maxY) / 2, maxY]) {
    brightness = Math.max(brightness, sceneryBrightness(fog, x + dx, y + dy));
    if (brightness === 1) return 1;
  }
  return brightness;
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
