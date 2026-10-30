// ------------------------------------------------------------
// Module: footprint
// Purpose: The squares a figure stands on. An odd body sits on
//          a square center. An even body sits on a corner.
//          This matches the server's snap.
// ------------------------------------------------------------

export function placeAnchor(value: number, span: number, limit: number): number | null {
  if (!Number.isFinite(value)) return null;
  const size = Math.max(1, Math.floor(span));
  if (size % 2 === 1) {
    const cell = Math.floor(value);
    if (cell < 0 || cell >= limit) return null;
    return cell + 0.5;
  }
  const center = Math.round(value);
  const half = size / 2;
  if (center < half || center + half > limit + 0.001) return null;
  return center;
}

export function footprintCells(x: number, y: number, widthCells: number, depthCells: number, mapWidth: number, mapHeight: number) {
  const anchorX = placeAnchor(x, widthCells, mapWidth);
  const anchorY = placeAnchor(y, depthCells, mapHeight);
  if (anchorX == null || anchorY == null) return [];
  const cells: { x: number; y: number }[] = [];
  for (const cellY of axisCells(anchorY, depthCells)) {
    for (const cellX of axisCells(anchorX, widthCells)) {
      cells.push({ x: cellX, y: cellY });
    }
  }
  return cells;
}

function axisCells(center: number, span: number) {
  const size = Math.max(1, Math.floor(span));
  const start = Math.floor(center - size / 2);
  const cells = [];
  for (let step = 0; step < size; step += 1) cells.push(start + step);
  return cells;
}
