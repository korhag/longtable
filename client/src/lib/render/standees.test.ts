// ------------------------------------------------------------
// Module: standees.test
// Purpose: Opening one door must not rebuild the other objects.
// ------------------------------------------------------------

import * as THREE from 'three';
import { beforeAll, describe, expect, it } from 'vitest';

import type { Pack } from '../assets/catalog';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { StandeeLayer } from './standees';

const pack: Pack = {
  id: 'core',
  name: 'Core',
  pixelsPerCell: 128,
  filtering: 'linear',
  terrains: [],
  decals: [],
  walls: [],
  doors: [{ id: 'gate', name: 'Gate', wall: 'hedge' }],
  props: [{ id: 'oak', name: 'Oak', footprint: [2, 2], blocks: 'move' }],
  themes: [],
  baseUrl: '',
  contentHash: 'core',
};

function layout(open: boolean): MapLayout {
  return {
    width: 8,
    height: 8,
    terrains: ['core/grass'],
    cells: '',
    pieces: [
      { id: 'hedge', kind: 'wall', asset: 'core/hedge', x: 1, y: 2, open: false, axis: 'h' },
      { id: 'gate', kind: 'door', asset: 'core/gate', x: 2, y: 2, open, axis: 'h' },
    ],
    props: [
      { id: 'oak', asset: 'core/oak', x: 4, y: 4, mirror: false },
      { id: 'barrel', asset: 'core/barrel', x: 6, y: 1, mirror: false },
    ],
    decals: [],
    lights: [],
    seed: 1,
  };
}

describe('standees', () => {
  beforeAll(() => {
    if (typeof document !== 'undefined') return;
    Object.assign(globalThis, {
      document: { createElement: () => ({ width: 0, height: 0, getContext: () => null }) },
    });
  });

  it('keeps the other objects when one door opens', () => {
    const scene = new THREE.Scene();
    const layer = new StandeeLayer(scene);
    const root = scene.children[0] as THREE.Group;
    layer.sync(layout(false), [pack], [{ id: 'gate', open: false, a: [2, 2], b: [3, 2] }], null, true);
    const oak = root.children[0];
    const barrel = root.children[1];
    const door = root.children[3];
    expect(root.children).toHaveLength(4);
    layer.sync(layout(true), [pack], [{ id: 'gate', open: true, a: [2, 2], b: [3, 2] }], null, true);
    expect(root.children[0]).toBe(oak);
    expect(root.children[1]).toBe(barrel);
    expect(root.children[3]).not.toBe(door);
    expect(root.children).toHaveLength(4);
    layer.clear();
  });
});
