import { expect, it } from 'vitest';
import { motionFacing, motionPosition, walkMotion } from './motion';
it('follows route corners rather than cutting across an obstacle', () => {
  const motion = walkMotion([{ x: 0, y: 0, z: 0 }, { x: 0, y: 0, z: 3 }, { x: 3, y: 1, z: 3 }], 100);
  expect(motionFacing(motion, 350, 'north')).toBe('south');
  expect(motionFacing(motion, 850, 'north')).toBe('east');
  expect(motionPosition(motion, 350)).toMatchObject({ x: 0, z: 1.5, done: false });
  expect(motionPosition(motion, 850)).toMatchObject({ x: 1.5, z: 3, done: false });
  expect(motionPosition(motion, 1100)).toEqual({ x: 3, y: 1, z: 3, done: true });
});
