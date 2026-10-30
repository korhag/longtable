import { describe, expect, it } from 'vitest';

import { pictureIndex, tileUv, tileVariant, nextStoredVariant } from './tileVariant';

describe('tile coverage', () => {
  it('puts the center of a cell at the center of its picture', () => {
    expect(tileUv(0.5, 0.5)).toEqual([0.5, 0.5]);
    expect(tileUv(4.5, 2.25)).toEqual([0.5, 0.25]);
  });

  it('does not stretch one picture across four cells', () => {
    expect(tileUv(3.9, 1.1)[0]).toBeGreaterThan(0.8);
    expect(tileUv(4.1, 1.1)[0]).toBeLessThan(0.2);
  });

  it('keeps a variant when only the inputs match', () => {
    const once = tileVariant(7, 3, 4, 'demo/floor', 3);
    const twice = tileVariant(7, 3, 4, 'demo/floor', 3);
    expect(once).toBe(twice);
    expect(once).toBeGreaterThanOrEqual(0);
    expect(once).toBeLessThan(3);
    expect(tileVariant(7, 3, 4, 'demo/floor', 1)).toBe(0);
  });

  it('keeps a chosen picture until the cycle returns to the seed', () => {
    const seeded = tileVariant(7, 3, 4, 'demo/floor', 3);
    expect(pictureIndex(7, 3, 4, 'demo/floor', 3, 0)).toBe(seeded);
    expect(pictureIndex(7, 3, 4, 'demo/floor', 3, 2)).toBe(1);
    expect(pictureIndex(7, 3, 4, 'demo/floor', 3, 9)).toBe(seeded);
    let stored = 0;
    for (let step = 0; step < 3; step += 1) stored = nextStoredVariant(7, 3, 4, 'demo/floor', 3, stored);
    expect(stored).toBe(0);
    expect(nextStoredVariant(7, 3, 4, 'demo/floor', 1, 4)).toBe(0);
  });
});
