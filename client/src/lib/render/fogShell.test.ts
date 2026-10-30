import { describe, expect, it } from 'vitest';
import * as THREE from 'three';

import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { FOG_LIFT } from './levels';
import { fogShell } from './terrain';

function layout(): MapLayout {
  const levels = new Uint8Array(16);
  levels[1 * 4 + 1] = 2;
  return {
    width: 4,
    height: 4,
    terrains: ['ruin/grass'],
    cells: encodeCells(new Uint8Array(16)),
    pieces: [],
    props: [],
    decals: [],
    lights: [],
    levels: encodeCells(levels),
    seed: 1,
    stairs: [],
  };
}

function hit(mesh: THREE.Mesh, origin: [number, number, number], direction: [number, number, number]) {
  const ray = new THREE.Raycaster(new THREE.Vector3(...origin), new THREE.Vector3(...direction).normalize());
  return ray.intersectObject(mesh, false)[0];
}

describe('fog on raised ground', () => {
  it('covers a raised top and the cliff, using the higher square', () => {
    const shell = fogShell(4, 4, layout());
    expect(shell.lifted).toBe(true);
    const mesh = new THREE.Mesh(shell.geometry);
    mesh.updateMatrixWorld(true);

    const top = hit(mesh, [1.5, 5, 1.5], [0, -1, 0]);
    expect(top?.point.y).toBeCloseTo(2 + FOG_LIFT);
    expect(Math.floor((top?.uv?.x ?? 0) * 4)).toBe(1);
    expect(Math.floor((1 - (top?.uv?.y ?? 0)) * 4)).toBe(1);

    const flat = hit(mesh, [0.5, 5, 0.5], [0, -1, 0]);
    expect(flat?.point.y).toBeCloseTo(FOG_LIFT);

    const cliff = hit(mesh, [3, 1, 1.5], [-1, 0, 0]);
    expect(cliff?.point.x).toBeCloseTo(2);
    expect(cliff?.point.y).toBeCloseTo(1);
    expect(Math.floor((cliff?.uv?.x ?? 0) * 4)).toBe(1);
    expect(Math.floor((1 - (cliff?.uv?.y ?? 1)) * 4)).toBe(1);
  });

  it('keeps a flat map on the ground plane', () => {
    const flat = layout();
    flat.levels = undefined;
    const shell = fogShell(4, 4, flat);
    expect(shell.lifted).toBe(false);
    const mesh = new THREE.Mesh(shell.geometry);
    mesh.updateMatrixWorld(true);
    const hitPoint = hit(mesh, [1.5, 5, 1.5], [0, -1, 0]);
    expect(hitPoint?.point.y).toBeCloseTo(0);
  });
});
