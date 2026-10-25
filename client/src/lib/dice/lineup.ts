// ------------------------------------------------------------
// Module: lineup
// Purpose: Places for dice after they stop tumbling. Each slot
//          is at least `spacing` away from the others, in
//          centered rows.
// ------------------------------------------------------------

export interface Slot {
  x: number;
  z: number;
}

export function lineUpSlots(count: number, perRow = 6, spacing = 1.35): Slot[] {
  if (count <= 0) return [];
  const rows = Math.ceil(count / perRow);
  const slots: Slot[] = [];
  for (let index = 0; index < count; index += 1) {
    const row = Math.floor(index / perRow);
    const column = index % perRow;
    const inRow = Math.min(perRow, count - row * perRow);
    slots.push({
      x: (column - (inRow - 1) / 2) * spacing,
      z: (row - (rows - 1) / 2) * spacing,
    });
  }
  return slots;
}
