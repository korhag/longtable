import { describe, expect, it } from 'vitest';

import { tileUv, tileVariant } from './tileVariant';

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
});
