/** Sprite views describe the screen, not the diamond grid's axes. The fixed
 * 45° / 30° camera projects (dx, dz) to (dx+dz, (dz-dx)/2).
 * Four pictures approximate the nearest view; they cannot invent diagonal art.
 */
export function facingToward(dx: number, dz: number, previous = 'south'): string {
  if (!Number.isFinite(dx + dz) || Math.abs(dx) + Math.abs(dz) < .001) return previous;
  const right = dx + dz, down = (dz - dx) * .5;
  return Math.abs(right) >= Math.abs(down) ? (right >= 0 ? 'east' : 'west') : (down >= 0 ? 'south' : 'north');
}
