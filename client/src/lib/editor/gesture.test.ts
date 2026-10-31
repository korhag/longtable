import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { editor } from './draft.svelte';
import { decodeCells, encodeCells } from './cells';

beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal('window', { setTimeout, clearTimeout });
  editor.draft = { id: 'gesture-test', name: 'Gesture', updatedAt: 0, layout: {
    width: 12, height: 12, terrains: ['core/grass'], cells: encodeCells(new Uint8Array(144)), pieces: [], props: [], decals: [], lights: [],
  } };
});
afterEach(() => { vi.clearAllTimers(); vi.useRealTimers(); vi.unstubAllGlobals(); });

it('paints the whole fast pointer stroke when movement reports button -1, and undoes it together', () => {
  editor.tool = 'terrain'; editor.paintMode = 'brush'; editor.brush = 1; editor.terrainId = 'forest/road';
  editor.pointer('down', { x: 1.2, y: 1.2 }, 0);
  editor.pointer('move', { x: 8.2, y: 1.2 }, -1);
  editor.pointer('up', { x: 8.2, y: 1.2 }, 0);
  const cells = decodeCells(editor.draft!.layout.cells);
  for (let x = 1; x <= 8; x += 1) expect(cells[12 + x]).toBe(1);
  editor.undo();
  expect(Array.from(decodeCells(editor.draft!.layout.cells)).every((value) => value === 0)).toBe(true);
});

it('draws a closed room and preserves its existing door', () => {
  editor.tool = 'wall'; editor.wallMode = 'room'; editor.wallId = 'forest/cabin-wall';
  editor.draft!.layout.pieces = [{ id: 'door', x: 3, y: 2, axis: 'h', kind: 'door', asset: 'forest/cabin-door', open: true }];
  editor.pointer('down', { x: 2, y: 2 }, 0);
  editor.pointer('move', { x: 6, y: 6 }, -1);
  editor.pointer('up', { x: 6, y: 6 }, 0);
  expect(editor.draft!.layout.pieces).toHaveLength(16);
  expect(editor.draft!.layout.pieces.find((piece) => piece.id === 'door')?.open).toBe(true);
});
