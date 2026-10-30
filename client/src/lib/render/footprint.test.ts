import { describe, expect, it } from 'vitest';

import { footprintCells, placeAnchor } from './footprint';

describe('figure footprint', () => {
  it('puts a one-square body on a center and a two-square body on a corner', () => {
    expect(placeAnchor(3.2, 1, 8)).toBe(3.5);
    expect(placeAnchor(3.2, 2, 8)).toBe(3);
    expect(placeAnchor(0.2, 2, 8)).toBe(null);
    expect(footprintCells(3.2, 4.1, 2, 2, 8, 8)).toEqual([
      { x: 2, y: 3 },
      { x: 3, y: 3 },
      { x: 2, y: 4 },
      { x: 3, y: 4 },
    ]);
  });
});
