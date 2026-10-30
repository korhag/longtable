import { describe, expect, it } from 'vitest';

import { propCells } from './draft.svelte';

describe('prop footprint preview', () => {
  it('covers the squares from the clicked corner', () => {
    expect(propCells(3.2, 8.8, 2, 2, 12, 12)).toEqual([
      { x: 3, y: 8 },
      { x: 4, y: 8 },
      { x: 3, y: 9 },
      { x: 4, y: 9 },
    ]);
  });

  it('drops squares that leave the map', () => {
    expect(propCells(11.2, 11.4, 2, 2, 12, 12)).toEqual([{ x: 11, y: 11 }]);
  });
});
