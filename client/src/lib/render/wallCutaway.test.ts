import * as THREE from 'three';
import { expect, it } from 'vitest';
import { setWallCutaway } from './wallCutaway';

it('cuts a raised wall above its ground edge without squeezing its artwork and restores it', () => {
  const group = new THREE.Group();
  group.position.y = 2;
  const material = new THREE.MeshBasicMaterial();
  const wall = new THREE.Mesh(new THREE.PlaneGeometry(1, 2), material);
  group.add(wall);
  setWallCutaway(group, true);
  expect(group.scale.y).toBe(1);
  const plane = material.clippingPlanes![0]!;
  expect(plane.distanceToPoint(new THREE.Vector3(0, 2.1, 0))).toBeGreaterThan(0);
  expect(plane.distanceToPoint(new THREE.Vector3(0, 3, 0))).toBeLessThan(0);
  setWallCutaway(group, false);
  expect(material.clippingPlanes).toBeNull();
});
