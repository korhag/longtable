// ------------------------------------------------------------
// Module: seen
// Purpose: Whether a ground point is inside the player's fog.
//          The GM, and a build session, see everything.
// ------------------------------------------------------------

import { bitAt, decodeBits } from '../fog/bits';
import type { FogView } from '../net/bindings/FogView.ts';

export function seen(fog: FogView | null, x: number, y: number) {
  if (!fog || fog.mode === 'gm') return true;
  const bytes = decodeBits(fog.explored);
  const scale = fog.cellsPerSquare || 4;
  const fx = Math.floor((x + 0.5) * scale);
  const fy = Math.floor((y + 0.5) * scale);
  return bitAt(bytes, fy * fog.width + fx);
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
    const bytes = decodeBits(fog.visible);
    const scale = fog.cellsPerSquare || 4;
    const fx = Math.floor(token.x * scale);
    const fy = Math.floor(token.y * scale);
    return bitAt(bytes, fy * fog.width + fx);
  }
  return fog.visiblePolygons.some((polygon) => pointInPoly(token.x, token.y, polygon));
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
