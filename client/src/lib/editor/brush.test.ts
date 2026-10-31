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

import { floodCells, wallEdges } from './brush';

it('fills only the connected material at one elevation', () => {
  const ground = new Uint8Array([0, 0, 1, 0, 0, 1, 1, 1, 0]);
  const levels = new Uint8Array([0, 0, 0, 1, 0, 0, 0, 0, 0]);
  expect(floodCells(0.2, 0.3, 3, 3, ground, levels)).toEqual([{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 1 }]);
  expect(floodCells(-1, 0, 3, 3, ground)).toEqual([]);
});

it('makes a gap-free wall and a closed rectangle regardless of drag direction', () => {
  expect(wallEdges({ x: 2, y: 2 }, { x: 7, y: 3 }, 'line', 10, 10)).toEqual(Array.from({ length: 5 }, (_, i) => ({ x: i + 2, y: 2, axis: 'h' })));
  const room = wallEdges({ x: 6, y: 5 }, { x: 2, y: 2 }, 'room', 10, 10);
  expect(room).toHaveLength(14);
  expect(new Set(room.map((edge) => `${edge.x},${edge.y},${edge.axis}`)).size).toBe(14);
  expect(room).toContainEqual({ x: 2, y: 2, axis: 'v' });
  expect(room).toContainEqual({ x: 5, y: 5, axis: 'h' });
  expect(wallEdges({ x: 2, y: 2 }, { x: 2, y: 6 }, 'room', 10, 10)).toEqual([]);
});
