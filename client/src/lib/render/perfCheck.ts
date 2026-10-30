// ------------------------------------------------------------
// Module: perfCheck
// Purpose: Measure the real table engine on a small scene and a
//          stress scene, then switch scenes thirty times. The
//          page records the GPU and the drawing buffer. It does
//          not decide that the table is smooth.
// ------------------------------------------------------------

import type { Pack } from '../assets/catalog';
import type { DoorView } from '../net/bindings/DoorView.ts';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { SceneView } from '../net/bindings/SceneView.ts';
import type { TokenView } from '../net/bindings/TokenView.ts';
import { TableEngine, type View } from './engine';
import { objectStats } from './sceneObjects';
import { terrainStats } from './terrain';

const status = document.querySelector('#out');
function say(text: string) {
  if (status) status.textContent = text;
}

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

const floor = png(128, 128, '#c46a45');
const barrel = png(32, 48, '#8a5a32');
const bandit = png(32, 64, '#d4a656');
const frame = { file: bandit, sizePx: [32, 64] as [number, number], pivotPx: [16, 60] as [number, number] };
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
  contentHash: 'perf',
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
  }, {
    id: 'gate',
    name: 'Gate',
    kind: 'door',
    placement: 'edge',
    footprintCells: [1, 1],
    pixelsPerWorldUnit: 64,
    blocksMovement: true,
    blocksVision: true,
    renderLayers: [
      { kind: 'upright', file: barrel, sizePx: [32, 48], pivotPx: [16, 46], axis: 'h', state: 'closed' },
      { kind: 'upright', file: barrel, sizePx: [32, 48], pivotPx: [16, 46], axis: 'h', state: 'open' },
    ],
  }],
  cast: [{ id: 'bandit', name: 'Bandit', pixelsPerWorldUnit: 64, views: { north: frame, east: frame, south: frame, west: frame } }],
};

function cells(count: number) {
  let binary = '';
  for (let index = 0; index < count; index += 1) binary += '\0';
  return btoa(binary);
}

function layout(size: number, objectCount: number, salt: number): MapLayout {
  const props = [];
  for (let index = 0; index < objectCount; index += 1) {
    props.push({ id: `p${salt}-${index}`, asset: 'gate/barrel', x: index % size, y: Math.floor(index / size) % size, mirror: false });
  }
  return {
    width: size,
    height: size,
    terrains: ['gate/floor'],
    cells: cells(size * size),
    pieces: [],
    props,
    decals: [],
    lights: [],
    seed: 1,
  };
}

function sceneOf(id: string, size: number, objectCount: number, castCount: number, salt: number): View {
  const map = layout(size, objectCount, salt);
  const scene: SceneView = {
    id,
    name: id,
    width: size,
    height: size,
    imageUrl: null,
    walls: [],
    lights: [],
    layout: map,
    presentation: null,
    schemaVersion: 2,
    visualProfile: 'dimetric-30-v1',
  };
  const tokens: TokenView[] = [];
  for (let index = 0; index < castCount; index += 1) {
    tokens.push({
      id: `c${salt}-${index}`,
      name: `Cast ${index}`,
      x: (index % size) + 0.5,
      y: (Math.floor(index / size) % size) + 0.5,
      kind: 'disc',
      imageUrl: null,
      color: '#d4a656',
      ownerId: null,
      hidden: false,
      visionRadius: 6,
      grantsVision: true,
      characterId: null,
      castAsset: 'gate/bandit',
      facing: 'south',
    });
  }
  const fog: FogView = { cellsPerSquare: 1, width: size, height: size, explored: '', visiblePolygons: [], mode: 'gm' };
  const doors: DoorView[] = [];
  return { scene, tokens, doors, fog, role: 'gm', selectedId: null, targetId: null, hoverId: null, tool: 'select', packs: [pack], ghost: [], buildMode: false, viewId: id, cast: {} };
}

function wait(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

async function activeFrames(count: number) {
  const samples: number[] = [];
  let last = performance.now();
  for (let index = 0; index < count; index += 1) {
    engine.invalidate();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    const now = performance.now();
    samples.push(now - last);
    last = now;
  }
  return samples.slice(5);
}

function summarize(samples: number[]) {
  return {
    frames: samples.length,
    p50Ms: percentile(samples, 50),
    p95Ms: percentile(samples, 95),
    p99Ms: percentile(samples, 99),
  };
}

function percentile(samples: number[], rank: number) {
  const sorted = [...samples].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.max(0, Math.ceil((rank / 100) * sorted.length) - 1));
  return Math.round((sorted[index] ?? 0) * 10) / 10;
}

function paintedPixel(x: number, y: number, z: number) {
  return new Promise<Uint8Array>((resolve, reject) => {
    const timer = window.setTimeout(() => reject(new Error('the scene did not draw a barrel')), 2000);
    onPaint = () => {
      window.clearTimeout(timer);
      onPaint = () => {};
      try {
        resolve(readPixel(x, y, z));
      } catch (error) {
        reject(error);
      }
    };
    engine.invalidate();
  });
}

function readPixel(x: number, y: number, z: number) {
  const point = engine.projectClient(x, y, z);
  const canvas = host.querySelector('canvas');
  if (!point || !canvas) throw new Error('the barrel is outside the view');
  const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
  if (!gl) throw new Error('no drawing buffer');
  const rect = canvas.getBoundingClientRect();
  const px = Math.min(canvas.width - 1, Math.max(0, Math.round(((point.x - rect.left) / rect.width) * canvas.width)));
  const py = Math.min(canvas.height - 1, Math.max(0, Math.round((1 - (point.y - rect.top) / rect.height) * canvas.height) - 1));
  const pixel = new Uint8Array(4);
  gl.readPixels(px, py, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
  return pixel;
}

window.addEventListener('error', (event) => say(`PERF_FAIL ${event.message}`));

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
engine.start();
say('PERF_STEP engine');

try {
  const small = sceneOf('small', 20, 100, 8, 1);
  const beforeObjects = objectStats.rebuilds;
  const beforeTerrain = terrainStats.rebuilds;
  engine.sync(small);
  say('PERF_STEP small');
  let settledDraws = engine.sample().draws;
  let quiet = 0;
  for (let attempt = 0; attempt < 40 && quiet < 3; attempt += 1) {
    await wait(100);
    const draws = engine.sample().draws;
    if (draws === settledDraws) quiet += 1;
    else {
      settledDraws = draws;
      quiet = 0;
    }
  }
  const warmedDraws = engine.sample().draws;
  engine.sync(small);
  await wait(800);
  const idleDraws = engine.sample().draws;
  if (idleDraws !== warmedDraws) throw new Error(`idle drew ${idleDraws - warmedDraws} extra frame(s)`);
  const smallInstances = objectStats.instances;
  if (smallInstances !== 100) throw new Error(`the small scene has ${smallInstances} drawn objects`);
  const barrelPixel = await paintedPixel(10.5, 0.4, 0.5);
  const barrelDistance = Math.abs(barrelPixel[0] - 138) + Math.abs(barrelPixel[1] - 90) + Math.abs(barrelPixel[2] - 50);
  const floorDistance = Math.abs(barrelPixel[0] - 196) + Math.abs(barrelPixel[1] - 106) + Math.abs(barrelPixel[2] - 69);
  if (barrelDistance > 48 || barrelDistance >= floorDistance) {
    throw new Error(`a shared barrel did not draw (${barrelPixel[0]},${barrelPixel[1]},${barrelPixel[2]})`);
  }
  const moved = { ...small, tokens: small.tokens.map((token, index) => (index === 0 ? { ...token, x: token.x + 1, facing: 'east' } : token)) };
  const objectsBeforeMove = objectStats.rebuilds;
  const terrainBeforeMove = terrainStats.rebuilds;
  engine.sync(moved);
  await wait(100);
  if (terrainStats.rebuilds !== terrainBeforeMove) throw new Error('moving one actor rebuilt the terrain');
  if (objectStats.rebuilds !== objectsBeforeMove) throw new Error('moving one actor rebuilt objects');
  const smallFrames = await activeFrames(40);
  const base = small.scene;
  const layout = base?.layout;
  if (!base || !layout) throw new Error('the small scene has no layout');
  const withDoor: View = {
    ...small,
    doors: [{ id: 'gate-door', open: false, a: [1, 1], b: [2, 1] }],
    scene: {
      ...base,
      layout: {
        ...layout,
        pieces: [{ id: 'gate-door', kind: 'door' as const, asset: 'gate/gate', x: 1, y: 1, axis: 'h' as const, open: false }],
      },
    },
  };
  engine.sync(withDoor);
  await wait(100);
  const doorObjects = objectStats.rebuilds;
  const doorTerrain = terrainStats.rebuilds;
  engine.sync({ ...withDoor, doors: [{ id: 'gate-door', open: true, a: [1, 1], b: [2, 1] }] });
  await wait(100);
  if (terrainStats.rebuilds !== doorTerrain) throw new Error('opening a door rebuilt the terrain');
  const doorRebuilds = objectStats.rebuilds - doorObjects;
  if (doorRebuilds !== 1) throw new Error(`opening a door rebuilt ${doorRebuilds} objects`);
  const smallSample = {
    ...engine.sample(),
    terrainRebuilds: terrainStats.rebuilds - beforeTerrain,
    objectRebuilds: objectStats.rebuilds - beforeObjects,
    idleDraws,
    warmedDraws,
    frames: summarize(smallFrames),
    doorRebuilds,
  };

  const stress = sceneOf('stress', 50, 500, 50, 2);
  const stressObjects = objectStats.rebuilds;
  const stressTerrain = terrainStats.rebuilds;
  const stressStart = performance.now();
  engine.sync(stress);
  say('PERF_STEP stress');
  await wait(200);
  if (objectStats.instances !== 500) throw new Error(`the crowded scene has ${objectStats.instances} drawn objects`);
  const stressMs = performance.now() - stressStart;
  const stressFrames = await activeFrames(24);
  const stressSample = {
    ...engine.sample(),
    elapsedMs: Math.round(stressMs),
    terrainRebuilds: terrainStats.rebuilds - stressTerrain,
    objectRebuilds: objectStats.rebuilds - stressObjects,
    frames: summarize(stressFrames),
  };

  const first = sceneOf('a', 12, 20, 4, 3);
  const second = sceneOf('b', 16, 24, 4, 4);
  engine.sync(first);
  await wait(50);
  const memoryAtOne = engine.sample().textures;
  for (let cycle = 0; cycle < 30; cycle += 1) {
    engine.sync(cycle % 2 === 0 ? second : first);
  }
  await wait(50);
  const memoryAtThirty = engine.sample().textures;
  if (memoryAtThirty > memoryAtOne + 12) throw new Error(`textures grew from ${memoryAtOne} to ${memoryAtThirty}`);

  say(`PERF_OK ${JSON.stringify({
    small: smallSample,
    stress: stressSample,
    soak: { cycles: 30, texturesAfterOne: memoryAtOne, texturesAfterThirty: memoryAtThirty },
    note: 'These are measurements of this run. They are not a claim that every computer is smooth.',
  })}`);
} catch (error) {
  say(`PERF_FAIL ${error instanceof Error ? error.message : String(error)}`);
} finally {
  engine.dispose();
}
