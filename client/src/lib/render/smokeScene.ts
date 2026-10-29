// ------------------------------------------------------------
// Module: smokeScene
// Purpose: A headless page that draws the diagnostic fixture
//          with the fixed camera and reports shader failures.
// ------------------------------------------------------------

import * as THREE from 'three';

import { sceneBasis } from './cameraProfile';
import { castPixels, groundPixels, holePixels } from './fixtures/diagnostic';

const out = document.querySelector('#out');
const canvas = document.querySelector('#view');
const errors: string[] = [];
const previous = console.error;
console.error = (...args: unknown[]) => {
  errors.push(args.map(String).join(' '));
  previous(...args);
};

function texture(width: number, height: number, pixel: (x: number, y: number) => [number, number, number, number]) {
  const data = new Uint8Array(width * height * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [r, g, b, a] = pixel(x, y);
      const index = (y * width + x) * 4;
      data[index] = r;
      data[index + 1] = g;
      data[index + 2] = b;
      data[index + 3] = a;
    }
  }
  const map = new THREE.DataTexture(data, width, height);
  map.colorSpace = THREE.SRGBColorSpace;
  map.magFilter = THREE.NearestFilter;
  map.needsUpdate = true;
  return map;
}

function main() {
  if (!(canvas instanceof HTMLCanvasElement) || !(out instanceof HTMLElement)) return;
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: false });
  renderer.setSize(640, 360, false);
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x1a1612);
  const camera = new THREE.OrthographicCamera(-8, 8, 4.5, -4.5, 0.05, 200);
  const focus = new THREE.Vector3(2, 0, 2);
  const { forward, up } = sceneBasis();
  camera.position.set(focus.x - forward.x * 20, focus.y - forward.y * 20, focus.z - forward.z * 20);
  camera.up.set(up.x, up.y, up.z);
  camera.lookAt(focus);
  const ground = texture(128, 128, groundPixels);
  const cutout = texture(64, 96, holePixels);
  const cast = texture(32, 64, castPixels);
  const floor = new THREE.Mesh(
    new THREE.PlaneGeometry(4, 4),
    new THREE.MeshBasicMaterial({ map: ground }),
  );
  floor.rotation.x = -Math.PI / 2;
  floor.position.set(2, 0, 2);
  scene.add(floor);
  const raised = new THREE.Mesh(
    new THREE.BoxGeometry(0.6, 1, 0.6),
    new THREE.MeshBasicMaterial({ color: 0xe23b32 }),
  );
  raised.position.set(3.2, 0.5, 1.2);
  scene.add(raised);
  const hole = new THREE.Mesh(
    new THREE.PlaneGeometry(1, 1.5),
    new THREE.MeshBasicMaterial({ map: cutout, transparent: true, alphaTest: 0.5, side: THREE.DoubleSide }),
  );
  hole.position.set(1.2, 0.75, 2.4);
  scene.add(hole);
  const figure = new THREE.Mesh(
    new THREE.PlaneGeometry(0.6, 1.2),
    new THREE.MeshBasicMaterial({ map: cast, transparent: true, alphaTest: 0.5, side: THREE.DoubleSide }),
  );
  figure.position.set(2.2, 0.6, 2.8);
  scene.add(figure);
  const east = new THREE.Mesh(new THREE.BoxGeometry(0.35, 0.08, 0.08), new THREE.MeshBasicMaterial({ color: 0xc4453c }));
  east.position.set(3.4, 0.05, 2);
  const north = new THREE.Mesh(new THREE.BoxGeometry(0.08, 0.08, 0.35), new THREE.MeshBasicMaterial({ color: 0x7eb8a2 }));
  north.position.set(2, 0.05, 0.8);
  scene.add(east, north);
  renderer.compile(scene, camera);
  renderer.render(scene, camera);
  const gl = renderer.getContext();
  const glError = gl.getError();
  const southwest = camera.position.x < focus.x && camera.position.z > focus.z;
  if (errors.length === 0 && glError === 0 && southwest) out.textContent = 'SMOKE_OK';
  else out.textContent = `SMOKE_FAIL gl=${glError} southwest=${southwest} ${errors.join(' | ')}`;
}

try {
  main();
} catch (error) {
  if (out instanceof HTMLElement) out.textContent = `SMOKE_FAIL ${String(error)}`;
}
