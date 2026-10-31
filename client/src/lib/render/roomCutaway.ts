import type { MapLayout } from '../net/bindings/MapLayout.ts';

/** Architectural boundaries include open doors: opening a door does not turn a room outdoors. */
export function roomCutaway(layout: MapLayout | null, occupants: { x: number; y: number }[]): Set<string> {
  const lowered = new Set<string>();
  if (!layout || occupants.length === 0 || layout.pieces.length === 0) return lowered;
  const edges = new Set(layout.pieces.map((piece) => `${piece.axis}:${piece.x},${piece.y}`));
  const visited = new Set<number>();
  const index = (x: number, y: number) => y * layout.width + x;
  for (const occupant of occupants) {
    const ox = Math.floor(occupant.x);
    const oy = Math.floor(occupant.y);
    if (ox < 0 || oy < 0 || ox >= layout.width || oy >= layout.height || visited.has(index(ox, oy))) continue;
    const room = new Set<number>([index(ox, oy)]);
    const queue = [{ x: ox, y: oy }];
    let outdoors = false;
    for (let cursor = 0; cursor < queue.length; cursor += 1) {
      const { x, y } = queue[cursor]!;
      visited.add(index(x, y));
      const neighbors = [
        { x: x - 1, y, edge: `v:${x},${y}` },
        { x: x + 1, y, edge: `v:${x + 1},${y}` },
        { x, y: y - 1, edge: `h:${x},${y}` },
        { x, y: y + 1, edge: `h:${x},${y + 1}` },
      ];
      for (const next of neighbors) {
        if (edges.has(next.edge)) continue;
        if (next.x < 0 || next.y < 0 || next.x >= layout.width || next.y >= layout.height) {
          outdoors = true;
          continue;
        }
        const key = index(next.x, next.y);
        if (room.has(key)) continue;
        room.add(key);
        queue.push(next);
      }
    }
    if (outdoors) continue;
    // The fixed camera looks from the south-west. Keep the far walls as a backdrop,
    // and lower only the south and west boundaries of the occupied room.
    const contains = (x: number, y: number) => x >= 0 && y >= 0 && x < layout.width && y < layout.height && room.has(index(x, y));
    for (const piece of layout.pieces) {
      if (piece.kind !== 'wall') continue;
      const inside = piece.axis === 'h' ? contains(piece.x, piece.y - 1) : contains(piece.x, piece.y);
      const outside = piece.axis === 'h' ? contains(piece.x, piece.y) : contains(piece.x - 1, piece.y);
      if (inside && !outside) lowered.add(piece.id);
    }
  }
  return lowered;
}
