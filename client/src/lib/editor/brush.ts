// ------------------------------------------------------------
// Module: brush
// Purpose: The round set of cells under the cursor, and the
//          wall edge nearest a point.
// ------------------------------------------------------------

import type { EdgeAxis } from '../net/bindings/EdgeAxis.ts';

export const BRUSH_SIZES = [1, 3, 5, 9] as const;
export type BrushSize = (typeof BRUSH_SIZES)[number];

export interface Cell {
  x: number;
  y: number;
}

export function brushCells(x: number, y: number, size: number, width: number, height: number): Cell[] {
  const radius = Math.max(0, (size - 1) / 2);
  const cx = Math.floor(x);
  const cy = Math.floor(y);
  const cells: Cell[] = [];
  const reach = Math.ceil(radius);
  for (let dy = -reach; dy <= reach; dy += 1) {
    for (let dx = -reach; dx <= reach; dx += 1) {
      if (dx * dx + dy * dy > radius * radius + 1e-6) continue;
      const cellX = cx + dx;
      const cellY = cy + dy;
      if (cellX < 0 || cellY < 0 || cellX >= width || cellY >= height) continue;
      cells.push({ x: cellX, y: cellY });
    }
  }
  return cells;
}

export interface EdgeHit {
  x: number;
  y: number;
  axis: EdgeAxis;
}

export function nearestEdge(x: number, y: number, width: number, height: number): EdgeHit | null {
  if (width < 1 || height < 1) return null;
  const verticalX = Math.round(x);
  const horizontalY = Math.round(y);
  const vertical = Math.abs(x - verticalX);
  const horizontal = Math.abs(y - horizontalY);
  if (horizontal <= vertical) {
    return { x: clampIndex(Math.floor(x), width - 1), y: clampIndex(horizontalY, height), axis: 'h' };
  }
  return { x: clampIndex(verticalX, width), y: clampIndex(Math.floor(y), height - 1), axis: 'v' };
}

function clampIndex(value: number, max: number) {
  return Math.min(max, Math.max(0, value));
}
