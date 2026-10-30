import { describe, expect, it } from 'vitest';

import { cutoutPlacement } from './cutout';

describe('cutout placement', () => {
  it('puts a centered foot pivot on the root and keeps the picture upright', () => {
    const placed = cutoutPlacement([32, 48], [16, 46], 128);
    expect(placed.width).toBeCloseTo(32 / 128);
    expect(placed.offsetX).toBeCloseTo(0);
    expect(placed.offsetY).toBeGreaterThan(0);
    expect(placed.height).toBeGreaterThan(48 / 128);
  });

  it('leaves a center pivot unshifted', () => {
    const placed = cutoutPlacement([32, 48], [16, 24], 128);
    expect(placed.offsetX).toBeCloseTo(0);
    expect(placed.offsetY).toBeCloseTo(0);
  });
});
