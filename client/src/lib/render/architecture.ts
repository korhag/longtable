import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import type { ObjectDef } from '../assets/catalog';

type Structure = NonNullable<ObjectDef['structure']>;
const light = new THREE.Vector3(-0.35, 0.85, 0.4).normalize();

/** One retained mesh per edge, with geometry shared by every matching wall. */
export function structureGeometry(structure: Structure, open: boolean): THREE.BufferGeometry {
  const pieces: THREE.BufferGeometry[] = [];
  const { height, thickness } = structure;
  const add = (geometry: THREE.BufferGeometry, color = 0xffffff) => {
    const flat = geometry.toNonIndexed();
    geometry.dispose();
    const normals = flat.getAttribute('normal');
    const base = new THREE.Color(color);
    const colors: number[] = [];
    for (let n = 0; n < normals.count; n += 1) {
      const shade = 0.74 + 0.26 * Math.max(0, normals.getX(n) * light.x + normals.getY(n) * light.y + normals.getZ(n) * light.z);
      colors.push(base.r * shade, base.g * shade, base.b * shade);
    }
    flat.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
    pieces.push(flat);
  };
  const box = (w: number, h: number, d: number, x: number, y: number, z: number, color = 0xffffff, turn = 0) => {
    const geometry = new THREE.BoxGeometry(w, h, d);
    geometry.rotateY(turn);
    geometry.translate(x, y, z);
    add(geometry, color);
  };
  if (structure.profile === 'log-wall') {
    const rows = Math.ceil(height / (thickness * 0.9));
    const course = height / rows;
    for (let row = 0; row < rows; row += 1) {
      const geometry = new THREE.CylinderGeometry(thickness / 2, thickness / 2, 1.06, 8, 1, false);
      // Wood grain follows the log's length; a whole square grain tile wrapped
      // around each thin course creates sparkling rings at normal map zoom.
      const uv = geometry.getAttribute('uv');
      for (let index = 0; index < uv.count; index += 1) {
        const u = uv.getX(index), v = uv.getY(index);
        uv.setXY(index, v, u * 0.14 + (row % 5) * 0.16);
      }
      geometry.rotateZ(Math.PI / 2);
      geometry.translate(0, course * (row + 0.5), 0);
      add(geometry, row % 3 === 0 ? 0xf1ddbe : row % 3 === 1 ? 0xe4d3bd : 0xffffff);
    }
    // A solid sill grounds the wall and prevents light leaking through the log joints.
    box(1.02, 0.12, thickness * 0.92, 0, 0.06, 0, 0xbca487);
  } else {
    const post = Math.max(0.1, thickness * 0.55);
    const opening = height - 0.22;
    box(post, height, thickness, -0.5 + post / 2, height / 2, 0, 0xdac6a7);
    box(post, height, thickness, 0.5 - post / 2, height / 2, 0, 0xdac6a7);
    box(1, 0.2, thickness * 1.08, 0, height - 0.1, 0, 0xdac6a7);
    box(1, 0.045, thickness * 1.8, 0, 0.0225, 0, 0xc1b196);
    const width = 1 - post * 2 - 0.025;
    const angle = open ? Math.PI * 0.48 : 0;
    const hinge = -0.5 + post;
    const transform = (x: number, z: number) => ({ x: hinge + Math.cos(angle) * x + Math.sin(angle) * z, z: -Math.sin(angle) * x + Math.cos(angle) * z });
    const plankWidth = width / 5;
    for (let plank = 0; plank < 5; plank += 1) {
      const at = transform((plank + 0.5) * plankWidth, 0);
      box(plankWidth - 0.006, opening - 0.04, 0.07, at.x, opening / 2 + 0.02, at.z, plank % 2 ? 0xddd0b9 : 0xf1dfc1, angle);
    }
    for (const y of [0.4, opening - 0.38]) {
      const at = transform(width / 2, 0.044);
      box(width * 0.94, 0.065, 0.025, at.x, y, at.z, 0x48403a, angle);
    }
    const latch = transform(width - 0.1, 0.067);
    box(0.065, 0.15, 0.055, latch.x, opening * 0.47, latch.z, 0x3e3933, angle);
  }
  const geometry = mergeGeometries(pieces, false)!;
  pieces.forEach(piece => piece.dispose());
  geometry.userData.sharedArchitecture = true;
  geometry.computeBoundingSphere();
  return geometry;
}

export function structureTexture(texture: THREE.Texture) {
  if (texture.minFilter === THREE.LinearMipmapLinearFilter) return texture;
  texture.minFilter = THREE.LinearMipmapLinearFilter;
  texture.generateMipmaps = true;
  texture.needsUpdate = true;
  return texture;
}
