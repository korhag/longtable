import { expect, it, vi } from 'vitest';
import { decodeCells, encodeCells } from '../editor/cells';
import { cellLevel, sceneRise } from './levels';

vi.mock('../editor/cells', async importOriginal => {
  const original = await importOriginal<typeof import('../editor/cells')>();
  return { ...original, decodeCells: vi.fn(original.decodeCells) };
});

it('reuses the decoded elevation grid for moving feet and lights, and refreshes after a draft edit', () => {
  const layout = { width: 7, height: 1, levels: encodeCells(new Uint8Array([1, 0, 2, 1, 0, 2, 2])) };
  vi.mocked(decodeCells).mockClear();
  for (let sample = 0; sample < 1000; sample++) expect(cellLevel(layout, 2.5, 0.5)).toBe(2);
  expect(sceneRise(layout)).toBe(3);
  expect(decodeCells).toHaveBeenCalledTimes(1);
  layout.levels = encodeCells(new Uint8Array([1, 0, 0, 1, 0, 1, 0]));
  expect(cellLevel(layout, 2.5, 0.5)).toBe(0);
  expect(sceneRise(layout)).toBe(2);
  expect(decodeCells).toHaveBeenCalledTimes(2);
});
