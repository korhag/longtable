// ------------------------------------------------------------
// Module: loadCheck
// Purpose: A picture that is still loading when the scene
//          changes must not paint the new scene. The new
//          picture must appear on its own, without a click.
// ------------------------------------------------------------

import * as THREE from 'three';

import type { Pack } from '../assets/catalog';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { SceneView } from '../net/bindings/SceneView.ts';
import type { TokenView } from '../net/bindings/TokenView.ts';
import { TableEngine, type View } from './engine';

const status = document.querySelector('#out');
function say(text: string) {
  if (status) status.textContent = text;
}

type Held = { url: string; fire: () => void };
const held: Held[] = [];
const original = THREE.TextureLoader.prototype.load;
THREE.TextureLoader.prototype.load = function (this: THREE.TextureLoader, url: string, onLoad?: (texture: THREE.Texture) => void) {
  held.push({ url, fire: () => original.call(this, url, onLoad) });
  return new THREE.Texture();
} as typeof original;

function png(width: number, height: number, color: string) {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('no canvas');
  context.clearRect(0, 0, width, height);
  context.fillStyle = color;
  context.fillRect(8, 8, width - 16, height - 16);
  return canvas.toDataURL('image/png');
}

const floor = png(128, 128, '#3355aa');
const roseBarrel = png(32, 48, '#ff00aa');
const greenBarrel = png(32, 48, '#00cc66');
const roseCast = png(32, 64, '#ffcc00');
const greenCast = png(32, 64, '#3399ff');

function pack(id: string, barrel: string, cast: string): Pack {
  const frame = { file: cast, sizePx: [32, 64] as [number, number], pivotPx: [16, 60] as [number, number] };
  return {
    id,
    name: id,
    pixelsPerCell: 128,
    filtering: 'nearest',
    terrains: [],
    decals: [],
    walls: [],
    doors: [],
    props: [],
    themes: [],
    baseUrl: '',
    contentHash: id,
    tiles: [{ id: 'floor', name: 'Floor', kind: 'ground', sourceSizePx: [128, 128], coverageCells: [1, 1], files: [floor], transitionPolicy: 'rigid' }],
    objects: [{
      id: 'barrel',
      name: 'Barrel',
      kind: 'static',
      placement: 'cell',
      footprintCells: [1, 1],
      pixelsPerWorldUnit: 64,
      blocksMovement: true,
      renderLayers: [{ kind: 'upright', file: barrel, sizePx: [32, 48], pivotPx: [16, 46] }],
    }],
    cast: [{ id: 'bandit', name: 'Bandit', pixelsPerWorldUnit: 64, views: { north: frame, east: frame, south: frame, west: frame } }],
  };
}

const rose = pack('rose', roseBarrel, roseCast);
const green = pack('green', greenBarrel, greenCast);

function scene(id: string, source: Pack): View {
  const layout: MapLayout = {
    width: 12,
    height: 12,
    terrains: [`${source.id}/floor`],
    cells: btoa('\0'.repeat(144)),
    pieces: [],
    props: [{ id: 'barrel', asset: `${source.id}/barrel`, x: 1, y: 1, mirror: false }],
    decals: [],
    lights: [],
    seed: 1,
  };
  const sceneView: SceneView = {
    id,
    name: id,
    width: 12,
    height: 12,
    imageUrl: null,
    walls: [],
    lights: [],
    layout,
    presentation: null,
    schemaVersion: 2,
    visualProfile: 'dimetric-30-v1',
  };
  const token: TokenView = {
    id: 'ada',
    name: 'Ada',
    x: 6.5,
    y: 6.5,
    kind: 'disc',
    imageUrl: null,
    color: '#ffffff',
    ownerId: null,
    hidden: false,
    visionRadius: 6,
    grantsVision: true,
    characterId: null,
    castAsset: `${source.id}/bandit`,
    facing: 'south',
  };
  const fog: FogView = { cellsPerSquare: 1, width: 12, height: 12, explored: '', visiblePolygons: [], mode: 'gm' };
  return {
    scene: sceneView,
    tokens: [token],
    doors: [],
    fog,
    role: 'gm',
    selectedId: null,
    targetId: null,
    hoverId: null,
    tool: 'select',
    packs: [rose, green],
    ghost: [],
    placeSpan: null,
    buildMode: false,
    viewId: id,
    cast: {},
  };
}

function release(urls: string[]) {
  const wanted = new Set(urls);
  const ready = held.filter((item) => wanted.has(item.url));
  for (const item of ready) {
    held.splice(held.indexOf(item), 1);
    item.fire();
  }
  return ready.length;
}

const host = document.querySelector('#host') as HTMLElement;
let onPaint = () => {};
const engine = new TableEngine(host, {
  onMove: () => {},
  onSelect: () => {},
  onBrush: () => {},
  onToggleDoor: () => {},
  canDrag: () => false,
  onPaint: () => onPaint(),
});

function wait(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function nextPaint() {
  return new Promise<ReturnType<typeof sample>>((resolve, reject) => {
    let last = 'none';
    const timer = window.setTimeout(() => reject(new Error(`the new picture did not redraw on its own (${last})`)), 3000);
    onPaint = () => {
      try {
        const seen = sample();
        last = `${seen.barrel.join(',')}|${seen.body.join(',')}`;
        const barrelReady = distance(seen.barrel, [0, 204, 102]) <= 80;
        const bodyReady = distance(seen.body, [51, 153, 255]) <= 80;
        if (!barrelReady || !bodyReady) return;
        window.clearTimeout(timer);
        onPaint = () => {};
        resolve(seen);
      } catch (error) {
        window.clearTimeout(timer);
        onPaint = () => {};
        reject(error);
      }
    };
  });
}

function readPixel(x: number, y: number, z: number) {
  const point = engine.projectClient(x, y, z);
  const canvas = host.querySelector('canvas');
  if (!point || !canvas) throw new Error(`the sample ${x},${z} is outside the view`);
  const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
  if (!gl) throw new Error('no drawing buffer');
  const rect = canvas.getBoundingClientRect();
  const px = Math.min(canvas.width - 1, Math.max(0, Math.round(((point.x - rect.left) / rect.width) * canvas.width)));
  const py = Math.min(canvas.height - 1, Math.max(0, Math.round((1 - (point.y - rect.top) / rect.height) * canvas.height) - 1));
  const pixel = new Uint8Array(4);
  gl.readPixels(px, py, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
  return [pixel[0] ?? 0, pixel[1] ?? 0, pixel[2] ?? 0] as const;
}

function sample() {
  return { barrel: readPixel(1.5, 0.4, 1.5), body: readPixel(6.5, 1.25, 6.5) };
}

function paintSample() {
  return new Promise<ReturnType<typeof sample>>((resolve, reject) => {
    const timer = window.setTimeout(() => reject(new Error('the scene did not draw')), 3000);
    onPaint = () => {
      window.clearTimeout(timer);
      onPaint = () => {};
      try {
        resolve(sample());
      } catch (error) {
        reject(error);
      }
    };
    engine.invalidate();
  });
}

function distance(pixel: readonly number[], color: readonly number[]) {
  return Math.abs((pixel[0] ?? 0) - (color[0] ?? 0)) + Math.abs((pixel[1] ?? 0) - (color[1] ?? 0)) + Math.abs((pixel[2] ?? 0) - (color[2] ?? 0));
}

window.addEventListener('error', (event) => say(`LOAD_FAIL ${event.message}`));

engine.start();

try {
  const first = scene('rose-room', rose);
  const second = scene('green-room', green);
  engine.sync(first);
  if (held.length < 2) throw new Error('the first scene did not hold its pictures');
  engine.sync(second);
  const stale = release([roseBarrel, roseCast]);
  if (stale < 2) throw new Error('the first scene pictures were not still waiting');
  await wait(400);
  const early = await paintSample();
  if (distance(early.barrel, [255, 0, 170]) < 80) throw new Error(`the old barrel painted the new room (${early.barrel.join(',')})`);
  if (distance(early.body, [255, 204, 0]) < 80) throw new Error(`the old person painted the new room (${early.body.join(',')})`);
  const before = engine.sample().draws;
  const arrived = nextPaint();
  const fresh = release([greenBarrel, greenCast]);
  if (fresh < 2) throw new Error('the new room pictures were not waiting');
  const painted = await arrived;
  if (engine.sample().draws <= before) throw new Error('the new picture did not redraw without a click');
  const quiet = engine.sample().draws;
  const started = performance.now();
  for (let index = 0; index < 30; index += 1) engine.sync(second);
  const syncMs = Math.round(performance.now() - started);
  await wait(200);
  if (engine.sample().draws !== quiet) throw new Error('an unchanged snapshot drew the map again');
  if (!engine.projectClient(1.5, 0.4, 1.5)) throw new Error('the camera stopped answering during snapshots');
  say(`LOAD_OK ${JSON.stringify({ staleReleased: stale, freshReleased: fresh, barrel: painted.barrel, body: painted.body, unchangedSyncMs: syncMs })}`);
} catch (error) {
  say(`LOAD_FAIL ${error instanceof Error ? error.message : String(error)}`);
}
