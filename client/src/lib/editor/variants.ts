// ------------------------------------------------------------
// Module: variants
// Purpose: A square can keep one picture of its tile. An empty
//          byte lets the map seed choose, and painting the
//          square clears that choice.
// ------------------------------------------------------------

import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { pictureIndex, nextStoredVariant } from '../render/tileVariant';
import type { Cell } from './brush';
import { decodeCells, encodeCells } from './cells';

export function pinVariant(layout: MapLayout, x: number, y: number, asset: string, count: number): { layout: MapLayout; stored: number; shown: number } {
  const stored = nextStoredVariant(layout.seed ?? 1, x, y, asset, count, storedAt(layout, x, y));
  const next = writePin(layout, x, y, stored);
  return { layout: next, stored, shown: pictureIndex(layout.seed ?? 1, x, y, asset, count, stored) + 1 };
}

export function clearVariantPins(layout: MapLayout, cells: Cell[]): MapLayout {
  if (!layout.variants || cells.length === 0) return layout;
  const count = layout.width * layout.height;
  const grid = decodeCells(layout.variants);
  if (grid.length !== count) return { ...layout, variants: undefined };
  let changed = false;
  for (const cell of cells) {
    const index = cell.y * layout.width + cell.x;
    if (index >= 0 && index < grid.length && grid[index] !== 0) {
      grid[index] = 0;
      changed = true;
    }
  }
  if (!changed) return layout;
  const variants = grid.some((byte) => byte !== 0) ? encodeCells(grid) : undefined;
  return { ...layout, variants };
}

function storedAt(layout: MapLayout, x: number, y: number): number {
  if (!layout.variants) return 0;
  const count = layout.width * layout.height;
  const grid = decodeCells(layout.variants);
  if (grid.length !== count) return 0;
  const index = y * layout.width + x;
  return grid[index] ?? 0;
}

function writePin(layout: MapLayout, x: number, y: number, stored: number): MapLayout {
  const count = layout.width * layout.height;
  const grid = layout.variants && decodeCells(layout.variants).length === count ? decodeCells(layout.variants) : new Uint8Array(count);
  const index = y * layout.width + x;
  if (index < 0 || index >= grid.length) return layout;
  grid[index] = stored;
  const variants = grid.some((byte) => byte !== 0) ? encodeCells(grid) : undefined;
  return { ...layout, variants };
}
