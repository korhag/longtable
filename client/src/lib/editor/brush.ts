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

/** Four-connected fill never leaks across a different ground or elevation. */
export function floodCells(x: number, y: number, width: number, height: number, ground: Uint8Array, levels?: Uint8Array): Cell[] {
  const startX = Math.floor(x);
  const startY = Math.floor(y);
  if (startX < 0 || startY < 0 || startX >= width || startY >= height) return [];
  const origin = startY * width + startX;
  const visited = new Uint8Array(width * height);
  const queue = [origin];
  const result: Cell[] = [];
  visited[origin] = 1;
  for (let index = 0; index < queue.length; index += 1) {
    const cell = queue[index]!;
    const cx = cell % width;
    const cy = Math.floor(cell / width);
    result.push({ x: cx, y: cy });
    for (const [nx, ny] of [[cx - 1, cy], [cx + 1, cy], [cx, cy - 1], [cx, cy + 1]]) {
      if (nx! < 0 || ny! < 0 || nx! >= width || ny! >= height) continue;
      const next = ny! * width + nx!;
      if (visited[next] || ground[next] !== ground[origin] || (levels?.[next] ?? 0) !== (levels?.[origin] ?? 0)) continue;
      visited[next] = 1;
      queue.push(next);
    }
  }
  return result;
}

/** A drag generates every edge, even when pointer events skip squares. */
export function wallEdges(start: Cell, end: Cell, mode: 'line' | 'room', width: number, height: number): EdgeHit[] {
  const ax = clampIndex(Math.round(start.x), width);
  const ay = clampIndex(Math.round(start.y), height);
  const bx = clampIndex(Math.round(end.x), width);
  const by = clampIndex(Math.round(end.y), height);
  const edges: EdgeHit[] = [];
  if (mode === 'room') {
    if (ax === bx || ay === by) return [];
    for (let x = Math.min(ax, bx); x < Math.max(ax, bx); x += 1) {
      edges.push({ x, y: Math.min(ay, by), axis: 'h' }, { x, y: Math.max(ay, by), axis: 'h' });
    }
    for (let y = Math.min(ay, by); y < Math.max(ay, by); y += 1) {
      edges.push({ x: Math.min(ax, bx), y, axis: 'v' }, { x: Math.max(ax, bx), y, axis: 'v' });
    }
  } else if (Math.abs(bx - ax) >= Math.abs(by - ay)) {
    for (let x = Math.min(ax, bx); x < Math.max(ax, bx); x += 1) edges.push({ x, y: ay, axis: 'h' });
  } else {
    for (let y = Math.min(ay, by); y < Math.max(ay, by); y += 1) edges.push({ x: ax, y, axis: 'v' });
  }
  return edges;
}
