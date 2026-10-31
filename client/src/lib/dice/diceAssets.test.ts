import { describe, expect, it } from 'vitest';
import * as THREE from 'three';
import type { DieMesh } from '../net/bindings/DieMesh';
import { diceGeometry, disposeObjects } from './diceAssets';

const vertices: DieMesh['vertices'] = [[1,1,1],[1,-1,-1],[-1,1,-1],[-1,-1,1]];
const tetra: DieMesh = { sides:4, vertices, faces: vertices.map((vertex, skip) => ({
  indices:[0,1,2,3].filter(index => index !== skip), value:skip+1,
  normal:vertex.map(value => -value / Math.sqrt(3)) as [number,number,number],
})) };

describe('batched dice labels', () => {
  it('keeps all twelve d4 corner labels in one mesh with correctly bounded atlas UVs', () => {
    const { body, labels, faces } = diceGeometry(tetra);
    expect(body.getAttribute('position').count).toBe(12);
    expect(labels.getAttribute('position').count).toBe(12 * 6);
    expect(faces).toHaveLength(12);
    for (let value = 1; value <= 4; value++) expect(faces.filter(face => face.value === value)).toHaveLength(3);
    const uv = labels.getAttribute('uv');
    for (let i = 0; i < uv.count; i++) {
      expect(uv.getX(i)).toBeGreaterThan(0);
      expect(uv.getX(i)).toBeLessThan(1);
      expect(uv.getY(i)).toBeGreaterThan(0);
      expect(uv.getY(i)).toBeLessThan(1);
    }
    // Every plate faces outwards; combining triangles must not reverse winding.
    const positions = labels.getAttribute('position');
    for (let index = 0; index < faces.length; index++) {
      const a = new THREE.Vector3().fromBufferAttribute(positions, index * 6);
      const b = new THREE.Vector3().fromBufferAttribute(positions, index * 6 + 1);
      const c = new THREE.Vector3().fromBufferAttribute(positions, index * 6 + 2);
      const normal = b.sub(a).cross(c.sub(a)).normalize();
      expect(normal.dot(new THREE.Vector3(0,0,1).applyQuaternion(faces[index]!.rotation))).toBeCloseTo(1);
    }
    body.dispose(); labels.dispose();
  });

  it('releases shared resources once and leaves warm cache entries alive', () => {
    const geometry = new THREE.PlaneGeometry();
    const map = new THREE.Texture();
    const material = new THREE.MeshBasicMaterial({ map });
    const shared = new THREE.MeshBasicMaterial({ map });
    let freedGeometry = 0, freedTexture = 0, freedMaterial = 0;
    geometry.addEventListener('dispose', () => freedGeometry++);
    map.addEventListener('dispose', () => freedTexture++);
    material.addEventListener('dispose', () => freedMaterial++);
    disposeObjects([new THREE.Mesh(geometry, material), new THREE.Mesh(geometry, shared)], new Set([geometry]), new Set([map]));
    expect([freedGeometry, freedTexture, freedMaterial]).toEqual([0,0,1]);
    disposeObjects([new THREE.Mesh(geometry, shared)]);
    expect([freedGeometry, freedTexture]).toEqual([1,1]);
  });
});
