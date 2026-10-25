import { describe, expect, it } from 'vitest';

import { brushCells } from './brush';

describe('brush', () => {
  it('returns the center for a single cell and the cross for a size of three', () => {
    expect(brushCells(5.2, 5.8, 1, 20, 20)).toEqual([{ x: 5, y: 5 }]);
    const wide = brushCells(5.2, 5.2, 3, 20, 20).map((cell) => `${cell.x},${cell.y}`);
    expect(wide).toContain('5,5');
    expect(wide).toContain('6,5');
    expect(wide).toContain('5,4');
    expect(wide).not.toContain('7,5');
    expect(wide).not.toContain('6,6');
  });
});
