import { describe, expect, it } from 'vitest';

import { encodeCells } from './cells';
import { objectLevels, placementRepair, propMirror, stairRepair } from './placementRepair';

describe('placement repair', () => {
  it('names a figure past the edge and does not move them', () => {
    const note = placementRepair([{ name: 'Ada', x: 11.5, y: 2.5, footprint: [1, 1] }], { width: 8, height: 8 });
    expect(note).toBe('Ada is outside the new map. Move them onto it, then put the map on the table again.');
  });

  it('names a body standing on two heights', () => {
    const levels = new Uint8Array(36);
    levels[0] = 0;
    levels[1] = 2;
    const note = placementRepair([{ name: 'Brute', x: 1, y: 1, footprint: [2, 2] }], { width: 6, height: 6, levels: encodeCells(levels) });
    expect(note).toBe('Brute is standing across two levels. Move them onto one level, then put the map on the table again.');
  });

  it('stays quiet when everyone fits', () => {
    expect(placementRepair([{ name: 'Ada', x: 1.5, y: 1.5 }], { width: 8, height: 8 })).toBeNull();
  });

  it('names a figure in the trunk and not one under the crown', () => {
    const packs = [{
      id: 'ruin',
      name: 'Ruin',
      pixelsPerCell: 128,
      filtering: 'nearest' as const,
      terrains: [],
      decals: [],
      walls: [],
      doors: [],
      props: [],
      themes: [],
      baseUrl: '',
      objects: [{
        id: 'oak',
        name: 'Oak',
        kind: 'static',
        placement: 'cell',
        footprintCells: [2, 2] as [number, number],
        originCells: [1, 1] as [number, number],
        renderLayers: [],
        blocksMovement: true,
        collisionPolygon: [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]] as [number, number][],
      }, {
        id: 'hedge',
        name: 'Hedge',
        kind: 'static',
        placement: 'cell',
        footprintCells: [1, 1] as [number, number],
        renderLayers: [],
        blocksMovement: false,
      }],
    }];
    const layout = {
      width: 12,
      height: 12,
      props: [{ asset: 'ruin/oak', x: 3, y: 8 }, { asset: 'ruin/hedge', x: 6, y: 2 }],
    };
    const inside = placementRepair([{ name: 'Ada', x: 4, y: 9 }], layout, packs);
    expect(inside).toBe('Ada is standing in something on the new map. Move them clear of it, then put the map on the table again.');
    expect(placementRepair([{ name: 'Wren', x: 3.2, y: 8.2 }], layout, packs)).toBeNull();
    expect(placementRepair([{ name: 'Bram', x: 6.5, y: 2.5 }], layout, packs)).toBeNull();
  });

  it('names a table standing on two heights and one that fits', () => {
    const packs = [{
      id: 'ruin',
      name: 'Ruin',
      pixelsPerCell: 128,
      filtering: 'nearest' as const,
      terrains: [],
      decals: [],
      walls: [],
      doors: [],
      props: [],
      themes: [],
      baseUrl: '',
      objects: [{
        id: 'table',
        name: 'Table',
        kind: 'static',
        placement: 'cell',
        footprintCells: [2, 1] as [number, number],
        renderLayers: [],
      }],
    }];
    const levels = new Uint8Array(36);
    levels[1 * 6 + 2] = 1;
    const split = { width: 6, height: 6, levels: encodeCells(levels), props: [{ asset: 'ruin/table', x: 1, y: 1 }] };
    expect(objectLevels(split, packs)).toBe('Table at 1, 1 stands on two heights. Move it so the whole object sits on one level, then put the map on the table again.');
    expect(objectLevels({ ...split, props: [{ asset: 'ruin/table', x: 3, y: 1 }] }, packs)).toBeNull();
    expect(objectLevels({ width: 6, height: 6, props: [{ asset: 'ruin/table', x: 5, y: 1 }] }, packs)).toBe('Table at 5, 1 does not fit on the map. Move it so the whole object sits on one level, then put the map on the table again.');
  });

  it('does not flip an upright picture', () => {
    const packs = [{
      id: 'ruin',
      name: 'Ruin',
      pixelsPerCell: 128,
      filtering: 'nearest' as const,
      terrains: [],
      decals: [],
      walls: [],
      doors: [],
      props: [{ id: 'crate', name: 'Crate', footprint: [1, 1] as [number, number], blocks: 'move' as const, mirror: true }],
      themes: [],
      baseUrl: '',
      objects: [{
        id: 'oak',
        name: 'Oak',
        kind: 'static',
        placement: 'cell',
        footprintCells: [2, 2] as [number, number],
        renderLayers: [],
      }],
    }];
    expect(propMirror('ruin/oak', packs, true)).toEqual({ mirror: false, note: 'Oak cannot be mirrored. Upright art needs its own picture.' });
    expect(propMirror('ruin/crate', packs, true)).toEqual({ mirror: true, note: null });
  });

  it('names a stair after both squares are the same height', () => {
    const levels = new Uint8Array(25);
    levels[1 * 5 + 2] = 1;
    const layout = {
      width: 5,
      height: 5,
      levels: encodeCells(levels),
      stairs: [{ x: 1, y: 1, x2: 2, y2: 1 }],
    };
    expect(stairRepair(layout)).toBeNull();
    levels[1 * 5 + 2] = 0;
    expect(stairRepair({ ...layout, levels: encodeCells(levels) })).toBe('These stairs do not join neighboring squares one step apart: (1, 1) to (2, 1).');
  });
});
