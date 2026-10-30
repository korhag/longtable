// ------------------------------------------------------------
// Module: levels
// Purpose: The height of one map square. 0 is the floor, 1 a
//          step, 2 a rise. A missing grid is all floor.
// ------------------------------------------------------------

import { decodeCells } from '../editor/cells';

/** The ground mesh sits a hair above the tier. Feet use this so they rest on it. */
export const SUPPORT_CLEARANCE = 0.02;

/** Fog sits above the ground paint and below a figure's feet. */
export const FOG_LIFT = 0.01;

export function cellLevel(layout: { width: number; height: number; levels?: string | null } | null | undefined, x: number, y: number): number {
  if (!layout?.levels) return 0;
  let bytes: Uint8Array;
  try {
    bytes = decodeCells(layout.levels);
  } catch {
    return 0;
  }
  const cx = Math.floor(x);
  const cy = Math.floor(y);
  if (cx < 0 || cy < 0 || cx >= layout.width || cy >= layout.height) return 0;
  const value = bytes[cy * layout.width + cx] ?? 0;
  return value <= 2 ? value : 0;
}

export function supportHeight(layout: { width: number; height: number; levels?: string | null } | null | undefined, x: number, y: number): number {
  return cellLevel(layout, x, y) + SUPPORT_CLEARANCE;
}

export function tierLabel(level: number): string {
  if (level >= 2) return 'On a rise';
  if (level === 1) return 'On a step';
  return 'On the floor';
}

/** Highest square, plus one world unit for a standing figure. */
export function sceneRise(layout: { width: number; height: number; levels?: string | null } | null | undefined): number {
  if (!layout?.levels || layout.width < 1 || layout.height < 1) return 1;
  let bytes: Uint8Array;
  try {
    bytes = decodeCells(layout.levels);
  } catch {
    return 1;
  }
  const count = layout.width * layout.height;
  let highest = 0;
  for (let index = 0; index < count && index < bytes.length; index += 1) {
    const value = bytes[index] ?? 0;
    if (value > highest && value <= 2) highest = value;
  }
  return highest + 1;
}
