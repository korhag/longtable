import * as THREE from 'three';
import { expect, it } from 'vitest';
import type { Pack } from '../assets/catalog';
import type { MapLayout } from '../net/bindings/MapLayout';
import { structureGeometry } from './architecture';
import { SceneObjects } from './sceneObjects';

it('builds a full-height log wall in one shared, bounded geometry', () => {
  const geometry = structureGeometry({ profile: 'log-wall', height: 2.65, thickness: 0.24, texture: '' }, false);
  geometry.computeBoundingBox();
  expect(geometry.boundingBox!.max.y).toBeGreaterThan(2.6);
  expect(geometry.boundingBox!.max.y).toBeLessThan(2.7);
  expect(geometry.boundingBox!.max.x - geometry.boundingBox!.min.x).toBeCloseTo(1.06);
  expect(geometry.getAttribute('position').count).toBeLessThan(2_000);
  expect(geometry.userData.sharedArchitecture).toBe(true);
  geometry.dispose();
});

it('picks the tall door surface and leaves the opening clear when the slab swings inward', () => {
  const pack = { id: 'test', contentHash: '', objects: [{
    id: 'door', kind: 'door', placement: 'edge', footprintCells: [1, 1], renderLayers: [],
    structure: { profile: 'plank-door', height: 2.65, thickness: 0.24, texture: '' },
  }] } as unknown as Pack;
  const layout = { width: 3, height: 3, props: [], pieces: [{ id: 'entry', kind: 'door', asset: 'test/door', x: 1, y: 1, axis: 'h', open: false }] } as unknown as MapLayout;
  const objects = new SceneObjects(new THREE.Scene());
  const ray = new THREE.Raycaster(new THREE.Vector3(1.5, 1.9, 3), new THREE.Vector3(0, 0, -1));
  objects.sync(layout, [pack], [], null, true, () => {});
  expect(objects.doorAt(ray)).toBe('entry');
  layout.pieces[0]!.open = true;
  objects.sync(layout, [pack], [], null, true, () => {});
  expect(objects.doorAt(ray)).toBeNull();
  // The lintel remains a visible, usable handle even while the door is open.
  ray.set(new THREE.Vector3(1.5, 2.56, 3), new THREE.Vector3(0, 0, -1));
  expect(objects.doorAt(ray)).toBe('entry');
  objects.clear();
});
