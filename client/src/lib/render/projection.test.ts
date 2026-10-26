import { describe, expect, it } from 'vitest';

import { basis, CLASSIC_ELEVATION_DEG, panGround } from './projection';

describe('projection', () => {
  it('matches the fixed classic camera', () => {
    const { forward, up, right } = basis(0, CLASSIC_ELEVATION_DEG);
    expect(right.x).toBeCloseTo(1, 2);
    expect(right.y).toBeCloseTo(0, 2);
    expect(up.y).toBeCloseTo(0.6, 2);
    expect(up.z).toBeCloseTo(-0.8, 2);
    expect(forward.y).toBeCloseTo(-0.8, 2);
    expect(forward.z).toBeCloseTo(-0.6, 2);
  });

  it('pans the classic view the same way the old camera did', () => {
    const { right, up } = basis(0, CLASSIC_ELEVATION_DEG);
    const shift = panGround(right, up, 10, 8, 1);
    expect(shift.x).toBeCloseTo(-10, 4);
    expect(shift.z).toBeCloseTo(-8 / 0.8, 4);
  });

  it('keeps four quarter turns a full circle apart', () => {
    const turns = [0, 90, 180, 270].map((azimuth) => basis(azimuth, CLASSIC_ELEVATION_DEG).forward);
    expect(turns[0].z).toBeLessThan(0);
    expect(turns[1].x).toBeGreaterThan(0);
    expect(turns[2].z).toBeGreaterThan(0);
    expect(turns[3].x).toBeLessThan(0);
    const back = basis(360, CLASSIC_ELEVATION_DEG).forward;
    expect(back.x).toBeCloseTo(turns[0].x, 5);
    expect(back.z).toBeCloseTo(turns[0].z, 5);
  });
});
