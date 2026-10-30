// ------------------------------------------------------------
// Module: decals.test
// Purpose: A rug on a raised square sits on that square, and a
//          fog change does not rebuild the rugs that stayed.
// ------------------------------------------------------------

import * as THREE from 'three';
import { describe, expect, it } from 'vitest';

import type { Pack } from '../assets/catalog';
import { encodeCells } from '../editor/cells';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { DecalLayer } from './decals';
import { supportHeight } from './levels';

const pack: Pack = {
  id: 'ruin',
  name: 'Ruin',
  pixelsPerCell: 128,
  filtering: 'nearest',
  terrains: [],
  decals: [{ id: 'leaves', name: 'Leaves', size: [1, 1] }],
  walls: [],
  doors: [],
  props: [],
  themes: [],
  baseUrl: '',
  contentHash: 'ruin',
};

function layout(): MapLayout {
  const levels = new Uint8Array(16);
  levels[1 * 4 + 2] = 2;
  return {
    width: 4,
    height: 4,
    terrains: ['ruin/grass'],
    cells: '',
    pieces: [],
    props: [],
    decals: [
      { asset: 'ruin/leaves', x: 1.5, y: 1.5, scale: 1, angle: 0 },
      { asset: 'ruin/leaves', x: 2.5, y: 1.5, scale: 1, angle: 0 },
    ],
    lights: [],
    seed: 1,
    levels: encodeCells(levels),
  };
}

function fog(explored: string): FogView {
  return { cellsPerSquare: 1, width: 4, height: 4, explored, visiblePolygons: [], mode: 'player' };
}

describe('decals', () => {
  it('sits a raised rug on that square and keeps both when the fog changes', () => {
    const scene = new THREE.Scene();
    const layer = new DecalLayer(scene);
    const root = scene.children[0] as THREE.Group;
    const map = layout();
    layer.sync(map, [pack], fog(''), true);
    const floor = root.children[0] as THREE.Mesh;
    const raised = root.children[1] as THREE.Mesh;
    expect(floor.position.y).toBeCloseTo(supportHeight(map, 1.5, 1.5), 5);
    expect(raised.position.y).toBeCloseTo(2.02, 5);
    layer.sync(map, [pack], fog('AA=='), true);
    expect(root.children[0]).toBe(floor);
    expect(root.children[1]).toBe(raised);
    layer.clear();
    expect(root.children).toHaveLength(0);
  });
});
