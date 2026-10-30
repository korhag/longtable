// ------------------------------------------------------------
// Module: stairLanding
// Purpose: A click on a stair lands on one of the two squares
//          it joins. The nearer end wins. The exact middle
//          lands on the higher square.
// ------------------------------------------------------------

import type { StairLink } from '../net/bindings/StairLink.ts';
import { cellLevel } from './levels';

type StairLayout = {
  width: number;
  height: number;
  levels?: string | null;
  stairs?: StairLink[] | null;
};

export function stairLanding(layout: StairLayout | null | undefined, x: number, z: number): { x: number; y: number } | null {
  if (!layout?.stairs?.length) return null;
  let best: { dist: number; t: number; stair: StairLink } | null = null;
  for (const stair of layout.stairs) {
    const ax = stair.x + 0.5;
    const az = stair.y + 0.5;
    const bx = stair.x2 + 0.5;
    const bz = stair.y2 + 0.5;
    const dx = bx - ax;
    const dz = bz - az;
    const length = dx * dx + dz * dz;
    if (length < 1e-6) continue;
    const t = Math.min(1, Math.max(0, ((x - ax) * dx + (z - az) * dz) / length));
    const px = ax + dx * t - x;
    const pz = az + dz * t - z;
    const dist = Math.hypot(px, pz);
    if (dist > 0.4) continue;
    if (!best || dist < best.dist) best = { dist, t, stair };
  }
  if (!best) return null;
  const low = { x: best.stair.x + 0.5, y: best.stair.y + 0.5 };
  const high = { x: best.stair.x2 + 0.5, y: best.stair.y2 + 0.5 };
  if (best.t < 0.5) return low;
  if (best.t > 0.5) return high;
  const lowTier = cellLevel(layout, best.stair.x, best.stair.y);
  const highTier = cellLevel(layout, best.stair.x2, best.stair.y2);
  return highTier >= lowTier ? high : low;
}
