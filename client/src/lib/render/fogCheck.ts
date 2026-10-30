// ------------------------------------------------------------
// Module: fogCheck
// Purpose: Draw the fog sheet on a raised top and its cliff.
//          A blank or flat-only sheet fails the page.
//          This is a correctness check, not a frame time.
// ------------------------------------------------------------

import * as THREE from 'three';

import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { cameraDepth, sceneBasis } from './cameraProfile';
import { FOG_LIFT } from './levels';
import { fogShell } from './terrain';

const out = document.querySelector('#out');
const canvas = document.querySelector('#view') as HTMLCanvasElement;

function report(text: string) {
  if (out) out.textContent = text;
}

const levels = new Uint8Array(16);
levels[1 * 4 + 1] = 2;
const layout: MapLayout = {
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

const image = new Uint8Array(4 * 4 * 4);
for (let index = 0; index < 16; index += 1) {
  const raised = index === 1 * 4 + 1;
  image[index * 4] = raised ? 255 : 0;
  image[index * 4 + 1] = 0;
  image[index * 4 + 2] = raised ? 0 : 255;
  image[index * 4 + 3] = 255;
}
const texture = new THREE.DataTexture(image, 4, 4, THREE.RGBAFormat);
texture.magFilter = THREE.NearestFilter;
texture.minFilter = THREE.NearestFilter;
texture.colorSpace = THREE.NoColorSpace;
texture.flipY = true;
texture.needsUpdate = true;

const shell = fogShell(4, 4, layout);
const mesh = new THREE.Mesh(
  shell.geometry,
  new THREE.MeshBasicMaterial({ map: texture, transparent: true, depthWrite: true, depthTest: true, alphaTest: 0.2, side: THREE.DoubleSide }),
);

const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: false });
renderer.setPixelRatio(1);
renderer.setSize(256, 256, false);
renderer.setClearColor(0x000000, 1);
const scene = new THREE.Scene();
scene.add(mesh);

const { forward, up, right } = sceneBasis();
const depth = cameraDepth(4, 4, 2);
const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, depth.near, depth.far);
const focus = new THREE.Vector3(2, 0, 2);
camera.position.set(focus.x - forward.x * depth.pull, focus.y - forward.y * depth.pull, focus.z - forward.z * depth.pull);
camera.up.set(up.x, up.y, up.z);
camera.lookAt(focus);

let minRight = Infinity;
let maxRight = -Infinity;
let minUp = Infinity;
let maxUp = -Infinity;
for (const y of [0, 2.03]) {
  for (const x of [0, 4]) {
    for (const z of [0, 4]) {
      const dx = x - focus.x;
      const dy = y - focus.y;
      const dz = z - focus.z;
      const alongRight = dx * right.x + dy * right.y + dz * right.z;
      const alongUp = dx * up.x + dy * up.y + dz * up.z;
      minRight = Math.min(minRight, alongRight);
      maxRight = Math.max(maxRight, alongRight);
      minUp = Math.min(minUp, alongUp);
      maxUp = Math.max(maxUp, alongUp);
    }
  }
}
const pad = 1.15;
camera.left = minRight * pad;
camera.right = maxRight * pad;
camera.bottom = minUp * pad;
camera.top = maxUp * pad;
camera.updateProjectionMatrix();

renderer.render(scene, camera);

function sample(x: number, y: number, z: number) {
  const point = new THREE.Vector3(x, y, z).project(camera);
  const px = Math.round((point.x * 0.5 + 0.5) * 255);
  const py = Math.round((point.y * 0.5 + 0.5) * 255);
  const pixels = new Uint8Array(4);
  const gl = renderer.getContext();
  gl.readPixels(px, py, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  return { px, py, r: pixels[0] ?? 0, g: pixels[1] ?? 0, b: pixels[2] ?? 0 };
}

const top = sample(1.5, 2 + FOG_LIFT, 1.5);
const cliff = sample(1, 1, 1.5);
const flat = sample(0.5, FOG_LIFT, 0.5);
const red = (pixel: { r: number; b: number }) => pixel.r > 200 && pixel.b < 40;
const blue = (pixel: { r: number; b: number }) => pixel.b > 200 && pixel.r < 40;

if (!shell.lifted || !red(top) || !red(cliff) || !blue(flat)) {
  report(`FOG_FAIL top ${top.px},${top.py} ${top.r},${top.g},${top.b} cliff ${cliff.px},${cliff.py} ${cliff.r},${cliff.g},${cliff.b} flat ${flat.px},${flat.py} ${flat.r},${flat.g},${flat.b}`);
} else {
  report('FOG_OK');
}
