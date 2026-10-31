import { expect, it } from 'vitest';
import { facingToward } from './facing';
import { project } from './cameraProfile';

it('chooses artwork in screen space and opposite views for opposing actors', () => {
  const opposite: Record<string, string> = { east: 'west', west: 'east', north: 'south', south: 'north' };
  for (const [x, z] of [[1, 0], [0, 1], [-1, 0], [0, -1], [1, 1], [-1, -1], [1, -1], [-1, 1]]) {
    const screen = project({ x: x!, y: 0, z: z! }, { x: 0, y: 0, z: 0 }, { u: 0, v: 0 }, 1);
    const expected = Math.abs(screen.u) >= Math.abs(screen.v) ? (screen.u > 0 ? 'east' : 'west') : (screen.v > 0 ? 'south' : 'north');
    const facing = facingToward(x!, z!, 'south');
    expect(facing).toBe(expected);
    expect(facingToward(-x!, -z!, 'south')).toBe(opposite[facing]);
  }
  expect(facingToward(1, 1, 'north')).toBe('east');
  expect(facingToward(1, -1, 'south')).toBe('north');
  expect(facingToward(0, 0, 'west')).toBe('west');
});
