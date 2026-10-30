// ------------------------------------------------------------
// Module: terrainSide.test
// Purpose: A raised edge uses the pack's cliff picture, and a
//          two-step face stacks that picture twice.
// ------------------------------------------------------------

import { describe, expect, it } from 'vitest';

import type { Pack } from '../assets/catalog';
import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { sideBatches } from './terrain';

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

function layout(): MapLayout {
  return {
    width: 2,
    height: 2,
    terrains: ['ruin/grass'],
    cells: encodeCells(new Uint8Array(4)),
    pieces: [],
    props: [],
    decals: [],
    lights: [],
    seed: 1,
    levels: encodeCells(new Uint8Array([2, 0, 0, 0])),
  };
}

describe('terrain sides', () => {
  it('paints a two-step edge with the cliff picture, stacked', () => {
    const batches = sideBatches(layout(), [pack], new Uint8Array([2, 0, 0, 0]));
    const cliff = batches.find((batch) => batch.url?.endsWith('tiles/cliff.png'));
    expect(cliff?.filtering).toBe('nearest');
    const heights = cliff?.uvs.filter((_, index) => index % 2 === 1) ?? [];
    expect(Math.max(...heights)).toBe(2);
  });
});
