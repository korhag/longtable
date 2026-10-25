import { describe, expect, it } from 'vitest';

import { impacts } from './sound';

describe('dice impacts', () => {
  it('marks the moment a falling die starts to rise', () => {
    const frames = [
      { p: [0, 1, 0] as [number, number, number] },
      { p: [0, 0.4, 0] as [number, number, number] },
      { p: [0, 0.05, 0] as [number, number, number] },
      { p: [0, 0.2, 0] as [number, number, number] },
      { p: [0, 0.15, 0] as [number, number, number] },
    ];
    const hits = impacts(frames, 0.05);
    expect(hits.length).toBeGreaterThan(0);
    expect(hits[0]?.time).toBeCloseTo(0.1);
    expect(hits[0]?.strength).toBeGreaterThan(0);
  });
});