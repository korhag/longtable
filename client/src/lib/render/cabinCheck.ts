import * as THREE from 'three';
import { cachedImage, fileUrl, loadTexture, type Pack } from '../assets/catalog';
import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout';
import { structureGeometry, structureTexture } from './architecture';
import { sceneBasis } from './cameraProfile';
import { cutoutMaterial, orientUpright, uprightGeometry } from './cutout';
import { SceneLighting } from './lighting';
import { roomCutaway } from './roomCutaway';
import { SceneObjects } from './sceneObjects';
import { TerrainLayer } from './terrain';

const out = document.querySelector('#out')!;
const canvas = document.querySelector('#view') as HTMLCanvasElement;
const errors: string[] = [];
const previousError = console.error;
console.error = (...args: unknown[]) => { errors.push(args.map(String).join(' ')); previousError(...args); };
const pack = await fetch('/packs/forest/pack.json').then(response => response.json()) as Pack;
pack.baseUrl = '/packs/forest';
for (const key of ['terrains', 'decals', 'walls', 'doors', 'props', 'themes'] as const) pack[key] ??= [];

function camera(width: number, height: number, span: number, x: number, y: number, z: number) {
  const view = new THREE.OrthographicCamera(-span * width / height / 2, span * width / height / 2, span / 2, -span / 2, 0.05, 100);
  const { forward, up } = sceneBasis();
  view.position.set(x - forward.x * 35, y - forward.y * 35, z - forward.z * 35);
  view.up.set(up.x, up.y, up.z);
  view.lookAt(x, y, z);
  view.updateProjectionMatrix();
  return view;
}

/** The same geometry can be exported as transparent schema-2 fallback artwork. */
async function architecturePreviews() {
  const preview = new THREE.WebGLRenderer({ alpha: true, antialias: true });
  preview.setSize(192, 384, false);
  preview.outputColorSpace = THREE.SRGBColorSpace;
  preview.setClearColor(0, 0);
  const view = camera(192, 384, 3, 0, 0, 0);
  view.top = 2.75; view.bottom = -0.25; view.updateProjectionMatrix();
  const result: Record<string, string> = {};
  for (const id of ['cabin-wall', 'cabin-door']) {
    const definition = pack.objects!.find(object => object.id === id)!;
    const structure = definition.structure!;
    const texture = await loadTexture(fileUrl(pack, structure.texture)!, pack.filtering);
    if (!texture) throw new Error('wood texture missing');
    for (const layer of definition.renderLayers) {
      const scene = new THREE.Scene();
      const geometry = structureGeometry(structure, layer.state === 'open');
      const material = new THREE.MeshBasicMaterial({ map: structureTexture(texture), vertexColors: true });
      const mesh = new THREE.Mesh(geometry, material);
      if (layer.axis === 'v') mesh.rotation.y = Math.PI / 2;
      scene.add(mesh);
      preview.render(scene, view);
      result[layer.file] = preview.domElement.toDataURL('image/png');
      material.dispose(); geometry.dispose();
    }
  }
  preview.dispose();
  return result;
}
(window as unknown as { architecturePreviews: typeof architecturePreviews }).architecturePreviews = architecturePreviews;

try {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  renderer.setPixelRatio(1); renderer.setSize(1200, 900, false);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.localClippingEnabled = true;
  renderer.setClearColor(0x100e0c);
  const scene = new THREE.Scene(), view = camera(1200, 900, 11.4, 6, 0.8, 4.8);
  const cells = new Uint8Array(120), levels = new Uint8Array(120);
  for (let z = 0; z < 10; z += 1) for (let x = 0; x < 12; x += 1) {
    if (x >= 1 && x < 8 && z >= 1 && z < 8) cells[z * 12 + x] = 2;
    if (x >= 10) levels[z * 12 + x] = 1;
    if (z === 8 && x >= 8) cells[z * 12 + x] = 1;
  }
  const layout: MapLayout = { width: 12, height: 10, terrains: ['forest/forest-floor', 'forest/road', 'forest/cabin-floor'], cells: encodeCells(cells), levels: encodeCells(levels), pieces: [], props: [], decals: [], lights: [], stairs: [{ id: 'stairs', asset: 'forest/stairs', x: 9, y: 8, x2: 10, y2: 8 }] };
  for (let n = 1; n < 8; n += 1) for (const edge of [{ x: n, y: 1, axis: 'h' }, { x: n, y: 8, axis: 'h' }, { x: 1, y: n, axis: 'v' }, { x: 8, y: n, axis: 'v' }] as const) {
    const door = edge.axis === 'h' && edge.y === 8 && edge.x === 4;
    layout.pieces.push({ ...edge, id: `${edge.axis}:${edge.x},${edge.y}`, kind: door ? 'door' : 'wall', asset: `forest/cabin-${door ? 'door' : 'wall'}`, open: false });
  }
  for (const [asset, x, y] of [['bed', 1, 2], ['desk', 5, 2], ['shelf', 5, 1], ['shelf', 5, 6], ['barrel', 1, 6], ['barrel', 2, 6]] as const) {
    layout.props.push({ id: `${asset}:${x},${y}`, asset: `forest/cabin-${asset}`, x, y, mirror: false });
  }
  const terrain = new TerrainLayer(scene), objects = new SceneObjects(scene), lighting = new SceneLighting(scene);
  const draw = () => { lighting.sync({ layout }, null, true); lighting.update(0, { x: 6, z: 5 }); lighting.bindScene(scene); renderer.render(scene, view); };
  await Promise.all(pack.tiles!.flatMap(tile => tile.files).map(file => cachedImage(fileUrl(pack, file)!)));
  terrain.sync(layout, [pack], draw);
  objects.sync(layout, [pack], [], null, true, draw, roomCutaway(layout, [{ x: 4.5, y: 4.5 }]));
  const cast = pack.cast!.find(cast => cast.id === 'bandit-captain')!;
  const frame = cast.views.south;
  const material = cutoutMaterial(await loadTexture(fileUrl(pack, frame.file)!, pack.filtering), 0xffffff, false);
  const figure = new THREE.Mesh(uprightGeometry(frame.sizePx, frame.pivotPx, cast.pixelsPerWorldUnit ?? 128), material);
  orientUpright(figure); figure.position.set(4.5, 0.02, 4.5); scene.add(figure);
  await new Promise(resolve => setTimeout(resolve, 1000));
  draw();
  (window as unknown as { showStairs: () => void }).showStairs = () => renderer.render(scene, camera(1200, 900, 4.5, 9.8, 0.65, 8.5));
  (window as unknown as { cabinGroundOnly: () => void }).cabinGroundOnly = () => { objects.clear(); scene.remove(figure); draw(); };
  if (errors.length) throw new Error(errors.join('\n'));
  if (renderer.info.render.calls > 50) throw new Error(`too many draws: ${renderer.info.render.calls}`);
  out.textContent = `CABIN_OK: full-height log walls, opening at character scale, cutaway interior, seven solid treads; ${renderer.info.render.calls} draws, ${renderer.info.render.triangles} triangles`;
} catch (error) { out.textContent = 'CABIN_FAIL: ' + String(error); }
