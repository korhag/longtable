// ------------------------------------------------------------
// Module: sceneCheck
// Purpose: Draw the real terrain shader and one upright cutout.
//          A blank buffer or a shader error fails the page.
//          This is a correctness check, not a performance result.
// ------------------------------------------------------------

import * as THREE from 'three';

import type { Pack } from '../assets/catalog';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { cameraDepth, sceneBasis } from './cameraProfile';
import { cutoutMaterial, orientUpright, uprightGeometry } from './cutout';
import { TerrainLayer } from './terrain';
import { checkOrganicTransitions } from './organicCheck';
import { checkTerrainOrientation } from './terrainOrientationCheck';

const out = document.querySelector('#out');
const canvas = document.querySelector('#view') as HTMLCanvasElement;

function report(text: string) {
  if (out) out.textContent = text;
}

const errors: string[] = [];
const previous = console.error;
console.error = (...args: unknown[]) => {
  errors.push(args.map(String).join(' '));
  previous(...args);
};

function png(width: number, height: number, paint: (context: CanvasRenderingContext2D) => void) {
  const source = document.createElement('canvas');
  source.width = width;
  source.height = height;
  const context = source.getContext('2d');
  if (!context) throw new Error('no canvas');
  paint(context);
  return source.toDataURL('image/png');
}

const floor = png(128, 128, (context) => {
  context.fillStyle = '#e23b2f';
  context.fillRect(0, 0, 128, 128);
});
const tree = png(64, 96, (context) => {
  context.clearRect(0, 0, 64, 96);
  context.fillStyle = '#2f8f4e';
  context.fillRect(16, 8, 32, 70);
});

const pack: Pack = {
  id: 'gate',
  name: 'Gate',
  pixelsPerCell: 128,
  filtering: 'nearest',
  terrains: [],
  decals: [],
  walls: [],
  doors: [],
  props: [],
  themes: [],
  baseUrl: '',
  contentHash: 'scene-check',
  tiles: [
    {
      id: 'floor',
      name: 'Floor',
      kind: 'ground',
      sourceSizePx: [128, 128],
      coverageCells: [1, 1],
      files: [floor],
      transitionPolicy: 'rigid',
    },
  ],
  objects: [
    {
      id: 'oak',
      name: 'Oak',
      kind: 'static',
      placement: 'cell',
      footprintCells: [1, 1],
      pixelsPerWorldUnit: 64,
      renderLayers: [{ kind: 'upright', file: tree, sizePx: [64, 96], pivotPx: [32, 90] }],
    },
  ],
};

const layout: MapLayout = {
  width: 2,
  height: 2,
  terrains: ['gate/floor'],
  cells: btoa(String.fromCharCode(0, 0, 0, 0)),
  pieces: [],
  props: [],
  decals: [],
  lights: [],
  seed: 1,
};

const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: false });
renderer.setPixelRatio(1);
renderer.setSize(256, 256, false);
renderer.setClearColor(0x100e0c, 1);
const scene = new THREE.Scene();
const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.05, 80);
const { forward, up } = sceneBasis();
const depth = cameraDepth(2, 2, 2);
camera.position.set(1 - forward.x * depth.pull, -forward.y * depth.pull, 1 - forward.z * depth.pull);
camera.up.set(up.x, up.y, up.z);
camera.lookAt(1, 0, 1);
camera.near = depth.near;
camera.far = depth.far;
camera.updateProjectionMatrix();

const terrain = new TerrainLayer(scene);
let paints = 0;
let transitionsReady = false;
void Promise.all([checkOrganicTransitions(), checkTerrainOrientation()]).then(() => { transitionsReady = true; draw(); }).catch((error) => report(`SCENE_FAIL ${String(error)}`));
terrain.sync(layout, [pack], () => {
  paints += 1;
  draw();
});

const object = pack.objects?.[0];
const layer = object?.renderLayers[0];
if (object && layer) {
  const mesh = new THREE.Mesh(uprightGeometry(layer.sizePx, layer.pivotPx, object.pixelsPerWorldUnit ?? 64), cutoutMaterial(null, 0xffffff, false));
  orientUpright(mesh);
  mesh.position.set(1, 0, 1);
  scene.add(mesh);
  const image = new Image();
  image.onload = () => {
    const texture = new THREE.Texture(image);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.needsUpdate = true;
    (mesh.material as THREE.MeshBasicMaterial).map = texture;
    (mesh.material as THREE.MeshBasicMaterial).needsUpdate = true;
    draw();
  };
  image.src = tree;
}

function draw() {
  renderer.render(scene, camera);
  const pixels = new Uint8Array(4);
  const gl = renderer.getContext();
  gl.readPixels(128, 128, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  const shader = errors.find((line) => /shader|fragColor|gl_FragColor/i.test(line));
  if (shader) {
    report(`SCENE_FAIL ${shader}`);
    return;
  }
  if (paints === 0) return;
  if (pixels[0] < 180) {
    report(`SCENE_FAIL center is ${pixels[0]},${pixels[1]},${pixels[2]} after ${paints} paint(s)`);
    return;
  }
  if (transitionsReady) report('SCENE_OK');
}

draw();
