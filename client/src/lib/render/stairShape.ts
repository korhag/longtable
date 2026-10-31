import type { StairLink } from '../net/bindings/StairLink';

export const STAIR_WIDTH = 0.9;
export const STAIR_RUN = 0.96;
export const STAIR_STEPS = 7;
export const STAIR_LANDING = 0.55;
export type StairMap = { width: number; height: number; stairs?: StairLink[] | null };
export type StairFrame = { x: number; z: number; dx: number; dz: number; low: number; high: number };
type LevelAt = (x: number, y: number) => number;

/** The flight sits on the lower square, ending at the cliff, never inside it. */
export function stairFrame(stair: StairLink, levelAt: LevelAt): StairFrame | null {
  const a = levelAt(stair.x, stair.y), b = levelAt(stair.x2, stair.y2);
  if (Math.abs(a - b) !== 1 || Math.abs(stair.x2 - stair.x) + Math.abs(stair.y2 - stair.y) !== 1) return null;
  const from = a < b ? { x: stair.x, z: stair.y } : { x: stair.x2, z: stair.y2 };
  const to = a < b ? { x: stair.x2, z: stair.y2 } : { x: stair.x, z: stair.y };
  const dx = to.x - from.x, dz = to.z - from.z;
  return { x: from.x + 0.5 - dx * 0.48, z: from.z + 0.5 - dz * 0.48, dx, dz, low: Math.min(a, b), high: Math.max(a, b) };
}

/** Exact tread support for feet and light sources. Returns null away from stairs. */
export function stairSurfaceHeight(layout: StairMap | null | undefined, x: number, z: number, levelAt: LevelAt): number | null {
  for (const stair of layout?.stairs ?? []) {
    const frame = stairFrame(stair, levelAt);
    if (!frame) continue;
    const rx = x - frame.x, rz = z - frame.z;
    const along = rx * frame.dx + rz * frame.dz;
    const across = rx * -frame.dz + rz * frame.dx;
    if (Math.abs(across) > STAIR_WIDTH / 2 || along < -0.12 || along > STAIR_RUN + STAIR_LANDING) continue;
    if (along <= 0) return frame.low;
    if (along >= STAIR_RUN) return frame.high;
    return frame.low + Math.ceil(along / STAIR_RUN * STAIR_STEPS) / STAIR_STEPS;
  }
  return null;
}
