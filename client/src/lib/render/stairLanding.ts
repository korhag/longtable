// ------------------------------------------------------------
// Module: stairLanding
// Purpose: A click on a stair lands on one of the two squares
//          it joins. The lower half of the physical flight
//          lands low; its upper half and landing go high.
// ------------------------------------------------------------

import type { StairLink } from '../net/bindings/StairLink.ts';
import { cellLevel } from './levels';
import { stairFrame, STAIR_LANDING, STAIR_RUN, STAIR_WIDTH } from './stairShape';

type StairLayout = {
  width: number;
  height: number;
  levels?: string | null;
  stairs?: StairLink[] | null;
};

export function stairLanding(layout: StairLayout | null | undefined, x: number, z: number): { x: number; y: number } | null {
  if (!layout?.stairs?.length) return null;
  let best: { dist: number; high: boolean; stair: StairLink } | null = null;
  for (const stair of layout.stairs) {
    const frame = stairFrame(stair, (x, y) => cellLevel(layout, x, y));
    if (!frame) continue;
    const rx = x - frame.x, rz = z - frame.z;
    const along = rx * frame.dx + rz * frame.dz;
    const dist = Math.abs(rx * -frame.dz + rz * frame.dx);
    if (dist > STAIR_WIDTH / 2 || along < -0.12 || along > STAIR_RUN + STAIR_LANDING) continue;
    if (!best || dist < best.dist) best = { dist, high: along >= STAIR_RUN / 2, stair };
  }
  if (!best) return null;
  const a = { x: best.stair.x + 0.5, y: best.stair.y + 0.5 };
  const b = { x: best.stair.x2 + 0.5, y: best.stair.y2 + 0.5 };
  const aIsLow = cellLevel(layout, best.stair.x, best.stair.y) < cellLevel(layout, best.stair.x2, best.stair.y2);
  return best.high === aIsLow ? b : a;
}
