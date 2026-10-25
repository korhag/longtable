import { describe, expect, it } from 'vitest';

import type { GridRule } from '../net/bindings/GridRule.ts';
import { measure, squareDistance } from './measure';

describe('grid measure', () => {
  it('counts every other diagonal as two squares', () => {
    expect(squareDistance(1, 1, 'alternate')).toBe(1);
    expect(squareDistance(2, 2, 'alternate')).toBe(3);
    expect(squareDistance(3, 1, 'alternate')).toBe(3);
    const feet: GridRule = { perSquare: 5, unit: 'ft', diagonal: 'alternate', ranges: [] };
    expect(measure(2, 2, feet).text).toBe('15 ft');
  });

  it('names a range band', () => {
    const grid: GridRule = {
      perSquare: 1,
      unit: '',
      diagonal: 'square',
      ranges: [{ name: 'Close', upTo: 6 }, { name: 'Far', upTo: 12 }],
    };
    expect(measure(4, 2, grid)).toMatchObject({ squares: 4, text: '4', range: 'Close' });
    expect(measure(8, 1, grid).range).toBe('Far');
  });
});
