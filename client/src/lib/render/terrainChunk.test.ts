// ------------------------------------------------------------
// Module: terrainChunk.test
// Purpose: Raising one square rebuilds the chunks on that edge,
//          not every chunk of a large map. A face belongs to
//          one chunk.
// ------------------------------------------------------------

import { describe, expect, it } from 'vitest';

import type { Pack } from '../assets/catalog';
import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { chunkIds, dirtyChunkIds, sideBatches } from './terrain';

const pack: Pack = {
  id: 'ruin',
  name: 'Ruin',
  pixelsPerCell: 128,
  filtering: 'nearest',
  terrains: [],
  decals: [],
  walls: [],
  doors: [],
  props: [],
  themes: [],
  baseUrl: '/packs/ruin',
  tiles: [
    { id: 'grass', name: 'Grass', kind: 'ground', sourceSizePx: [128, 128], coverageCells: [1, 1], files: ['tiles/grass.png'], transitionPolicy: 'rigid' },
    { id: 'cliff', name: 'Cliff', kind: 'side', sourceSizePx: [128, 128], coverageCells: [1, 1], files: ['tiles/cliff.png'], transitionPolicy: 'none' },
  ],
};

describe('terrain chunks', () => {
  it('rebuilds the nearby chunks when one square rises', () => {
    expect(chunkIds(20, 20)).toEqual(['0,0']);
    expect(chunkIds(200, 200)).toHaveLength(16);
    const next = new Uint8Array(200 * 200);
    next[10 * 200 + 10] = 2;
    expect(dirtyChunkIds(200, 200, null, next)).toEqual(['0,0']);
    const edge = new Uint8Array(200 * 200);
    edge[10 * 200 + 63] = 1;
    expect(dirtyChunkIds(200, 200, null, edge).sort()).toEqual(['0,0', '1,0']);
  });

  it('gives a shared edge to one chunk', () => {
    const levels = new Uint8Array(16);
    levels[1] = 2;
    const layout: MapLayout = {
      width: 4,
      height: 4,
      terrains: ['ruin/grass'],
      cells: encodeCells(new Uint8Array(16)),
      pieces: [],
      props: [],
      decals: [],
      lights: [],
      seed: 1,
      levels: encodeCells(levels),
    };
    const full = sideBatches(layout, [pack], levels).reduce((sum, batch) => sum + batch.positions.length, 0);
    const west = sideBatches(layout, [pack], levels, { x0: 0, y0: 0, x1: 2, y1: 4 }).reduce((sum, batch) => sum + batch.positions.length, 0);
    const east = sideBatches(layout, [pack], levels, { x0: 2, y0: 0, x1: 4, y1: 4 }).reduce((sum, batch) => sum + batch.positions.length, 0);
    expect(west + east).toBe(full);
    expect(west).toBeGreaterThan(0);
  });
});
