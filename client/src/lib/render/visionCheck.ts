// Browser fixture for the production fog texture and retained scenery renderer.
// Synthetic seat masks isolate rendering from server LOS (covered in Rust tests).
import * as THREE from 'three';
import type { Pack } from '../assets/catalog';
import type { FogView } from '../net/bindings/FogView';
import type { MapLayout } from '../net/bindings/MapLayout';
import { sceneBasis } from './cameraProfile';
import { drawFog } from './fogImage';
import { shadeScenery } from './fogShade';
import { FOG_LIFT } from './levels';
import { SceneObjects, objectStats } from './sceneObjects';
import { blockerBrightness, inSight } from './seen';
import { fogShell } from './terrain';

const pack: Pack = {
  id: 'fixture', name: 'Vision fixture', pixelsPerCell: 128, filtering: 'linear', baseUrl: '',
  terrains: [], decals: [], walls: [], doors: [], props: [], themes: [],
  objects: [
    { id: 'tree', name: 'Upright prop', kind: 'prop', placement: 'cell', footprintCells: [1, 1], renderLayers: [
      { file: '', kind: 'upright', sizePx: [96, 192], pivotPx: [48, 192] },
    ] },
    { id: 'wall', name: 'Log wall', kind: 'wall', placement: 'edge', footprintCells: [1, 1], renderLayers: [],
      structure: { profile: 'log-wall', height: 1.3, thickness: 0.2, texture: '' } },
    { id: 'door', name: 'Cabin door', kind: 'door', placement: 'edge', footprintCells: [1, 1], renderLayers: [],
      structure: { profile: 'plank-door', height: 1.3, thickness: 0.2, texture: '' } },
  ],
};

function encoded(test: (x: number, z: number) => boolean) {
  const bytes = new Uint8Array(32 * 32 / 8);
  for (let z = 0; z < 32; z++) for (let x = 0; x < 32; x++) {
    const index = z * 32 + x;
    if (test((x + 0.5) / 4, (z + 0.5) / 4)) bytes[index >> 3]! |= 1 << (7 - (index & 7));
  }
  return btoa(String.fromCharCode(...bytes));
}

function picture(east: boolean, open: boolean): FogView {
  const sx = east ? 6.5 : 1.5, sz = 3.5;
  const visible = encoded((x, z) => {
    if (x < 1 && z < 1) return false;
    const dx = x - sx, dz = z - sz;
    const t = (4 - sx) / dx;
    const wallZ = sz + t * dz;
    if (t > 0 && t < 1 && wallZ >= 2 && wallZ <= 6 && !(open && wallZ >= 3 && wallZ <= 4)) return false;
    const near = Math.max(0, Math.min(1, ((2.5 - sx) * dx + (2 - sz) * dz) / (dx * dx + dz * dz)));
    return Math.hypot(sx + near * dx - 2.5, sz + near * dz - 2) > 0.5;
  });
  return { cellsPerSquare: 4, width: 32, height: 32, mode: 'player', visiblePolygons: [],
    explored: encoded((x, z) => !(x < 1 && z < 1)), visible };
}

export function checkPlayerVision() {
  const scene = new THREE.Scene();
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(8, 8), new THREE.MeshBasicMaterial({ color: '#b2a88e' }));
  ground.rotation.x = -Math.PI / 2;
  ground.position.set(4, 0, 4);
  scene.add(ground);
  const boulder = new THREE.Mesh(new THREE.SphereGeometry(0.5, 12, 8), new THREE.MeshBasicMaterial({ color: '#aabb9a' }));
  boulder.position.set(2.5, 0.4, 2);
  scene.add(boulder);
  const layout: MapLayout = { width: 8, height: 8, terrains: [], cells: '', decals: [], lights: [], seed: 1,
    props: [{ id: 'far-tree', asset: 'fixture/tree', x: 6, y: 1, mirror: false }],
    pieces: [2, 3, 4, 5].map(y => ({ id: `wall-${y}`, asset: `fixture/${y === 3 ? 'door' : 'wall'}`,
      kind: y === 3 ? 'door' : 'wall', x: 4, y, axis: 'v', open: false })),
  };
  const objects = new SceneObjects(scene);
  const fogCanvas = document.createElement('canvas');
  const texture = new THREE.CanvasTexture(fogCanvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.generateMipmaps = false;
  texture.minFilter = THREE.LinearFilter;
  const shell = fogShell(8, 8, null);
  const fogMesh = new THREE.Mesh(shell.geometry, new THREE.MeshBasicMaterial({ map: texture, transparent: true,
    depthWrite: true, depthTest: true, alphaTest: 0.2, side: THREE.DoubleSide }));
  fogMesh.position.y = FOG_LIFT;
  scene.add(fogMesh);
  const canvas = document.createElement('canvas');
  canvas.style.cssText = 'display:block;width:min(100%,960px);height:auto';
  document.body.append(canvas);
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, preserveDrawingBuffer: true });
  renderer.setPixelRatio(1);
  renderer.setSize(960, 320, false);
  renderer.setClearColor('#14110c');
  renderer.setScissorTest(true);
  const camera = new THREE.OrthographicCamera(-6, 6, 4, -4, 0.1, 100);
  const { forward, up } = sceneBasis();
  camera.position.set(4 - forward.x * 20, 0.4 - forward.y * 20, 4 - forward.z * 20);
  camera.up.set(up.x, up.y, up.z);
  camera.lookAt(4, 0.4, 4);
  camera.updateMatrixWorld();

  const sample = (panel: number, x: number, y: number, z: number) => {
    const point = new THREE.Vector3(x, y, z).project(camera);
    const pixels = new Uint8Array(4);
    const gl = renderer.getContext();
    gl.readPixels(panel * 320 + Math.round((point.x * 0.5 + 0.5) * 319), Math.round((point.y * 0.5 + 0.5) * 319), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
    return pixels[0]!;
  };
  const readings: number[][] = [];
  let retained = false;
  for (const [panel, east, open] of [[0, false, false], [1, true, false], [2, false, true]] as const) {
    const fog = picture(east, open);
    const before = objectStats.rebuilds;
    for (const piece of layout.pieces) piece.open = open && piece.kind === 'door';
    objects.sync(layout, [pack], [], fog, false, () => {});
    if (panel === 1) retained = before === objectStats.rebuilds;
    shadeScenery(boulder, blockerBrightness(fog, 2.5, 2, [1, 1]));
    drawFog(fog, 8, 8, fogCanvas);
    texture.needsUpdate = true;
    renderer.setViewport(panel * 320, 0, 320, 320);
    renderer.setScissor(panel * 320, 0, 320, 320);
    renderer.render(scene, camera);
    readings.push([
      sample(panel, 1.5, FOG_LIFT, 6.5), sample(panel, 6.5, FOG_LIFT, 6.5),
      sample(panel, 0.5, FOG_LIFT, 0.5), sample(panel, 6.5, 1.2, 1.5),
      sample(panel, 6, FOG_LIFT, 3.5),
      sample(panel, 2.5, 0.7, 2),
    ]);
    // Enemy figures (and hence their silhouette passes) must not enter the scene through memory alone.
    if (!open && !east && inSight({ x: 6, y: 3.5, ownerId: 'enemy' }, fog, 'west')) throw new Error('Hidden actor leaked');
  }
  const [west, east, opened] = readings as [number[], number[], number[]];
  const ok = retained && west[0]! > 140 && west[1]! < 90 && west[2]! < 15 && west[3]! < 110
    && east[1]! > 140 && east[3]! > 220 && opened[4]! > 140 && west[4]! < 90 && west[5]! > 130;
  objects.clear();
  texture.dispose();
  renderer.dispose();
  if (!ok) throw new Error(`Player pictures ${JSON.stringify(readings)} retained=${retained}`);
  return 'VISION_OK west/east views, opaque unknown, dim memory/props, door reveal, retained scenery';
}
