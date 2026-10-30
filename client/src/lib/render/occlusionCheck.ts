// ------------------------------------------------------------
// Module: occlusionCheck
// Purpose: Draw the real outline pass and the real table camera.
//          A hidden figure must not be selectable, a transparent
//          sprite pixel must not be a hit, and a 200-square map
//          must keep its corners in front of the camera. The ruin
//          oak must outline a figure the server still sends, and
//          a figure left out of that view must not be a hover target.
//          A cut across the body must not draw a line through it.
//          Two overlapping figures keep separate outlines, and the
//          selected one is gold. Walking in front of the oak drops
//          the outline. A barrel outlines someone on the floor, including
//          a figure with no picture. The part of a body above a short
//          piece of scenery is left alone. A hole in the scenery
//          shows the figure and does not draw a line across them.
//          A cliff on this camera outlines only the covered part
//          of a taller figure.
// ------------------------------------------------------------

import * as THREE from 'three';

import type { Pack } from '../assets/catalog';
import { encodeCells } from '../editor/cells';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { SceneView } from '../net/bindings/SceneView.ts';
import type { TokenView } from '../net/bindings/TokenView.ts';
import type { ObjectDef } from '../assets/catalog';
import { cutoutMaterial } from './cutout';
import { TableEngine, type View } from './engine';
import { OcclusionOutlines } from './occlusion';

const status = document.querySelector('#out');
function say(text: string) {
  if (status) status.textContent = text;
}

window.addEventListener('error', (event) => say(`OCCLUSION_FAIL ${event.message}`));

function wait(ms: number) {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function canvasTexture(paint: (context: CanvasRenderingContext2D, canvas: HTMLCanvasElement) => void, width: number, height: number) {
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('no canvas');
  paint(context, canvas);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.needsUpdate = true;
  return texture;
}

function isCream(r: number, g: number, b: number) {
  return r > 160 && g > 150 && b > 120 && g > 100;
}

function tierPass() {
  const css = 160;
  const canvas = document.querySelector('#view') as HTMLCanvasElement;
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: false });
  renderer.setPixelRatio(1);
  renderer.setSize(css, css, false);
  renderer.setClearColor(0x100e0c, 1);
  const scene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-2, 2, 2, -2, 0.1, 20);
  camera.position.set(0, 0, 5);
  camera.lookAt(0, 0, 0);
  const scenery = new THREE.Mesh(new THREE.PlaneGeometry(2.4, 1.2), new THREE.MeshBasicMaterial({ color: 0x2f8f4e }));
  scenery.position.set(0, -0.7, 0.8);
  scene.add(scenery);
  const cast = new THREE.Mesh(new THREE.PlaneGeometry(1.2, 2.4), new THREE.MeshBasicMaterial({ color: 0x3a6ea5 }));
  cast.position.set(0, 0.4, 0);
  cast.userData.tokenId = 'ada';
  scene.add(cast);
  const outlines = new OcclusionOutlines();
  renderer.render(scene, camera);
  outlines.draw(renderer, scene, camera, [cast], null, [cast]);
  const pixels = new Uint8Array(css * css * 4);
  const gl = renderer.getContext();
  gl.readPixels(0, 0, css, css, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  let low = 0;
  let high = 0;
  let head = 0;
  for (let y = 0; y < css; y += 1) {
    for (let x = 0; x < css; x += 1) {
      const index = (y * css + x) * 4;
      const r = pixels[index] ?? 0;
      const g = pixels[index + 1] ?? 0;
      const b = pixels[index + 2] ?? 0;
      const worldY = (y / css) * 4 - 2;
      if (b > 120 && b > r + 30 && b > g) head += 1;
      if (!isCream(r, g, b)) continue;
      if (worldY < -0.15) low += 1;
      if (worldY > 0.45) high += 1;
    }
  }
  outlines.dispose();
  renderer.dispose();
  if (head < 20) throw new Error(`the raised part of the figure was not drawn (${head} blue pixels)`);
  if (low < 8) throw new Error(`the low scenery did not outline the covered part (${low} cream pixels)`);
  if (high > 0) throw new Error(`the part above the scenery was outlined (${high} cream pixels)`);
}

function holePass() {
  const css = 160;
  const canvas = document.querySelector('#view') as HTMLCanvasElement;
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: false });
  renderer.setPixelRatio(1);
  renderer.setSize(css, css, false);
  renderer.setClearColor(0x100e0c, 1);
  const scene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-2, 2, 2, -2, 0.1, 20);
  camera.position.set(0, 0, 5);
  camera.lookAt(0, 0, 0);
  const leaves = canvasTexture((context) => {
    context.clearRect(0, 0, 64, 64);
    context.fillStyle = '#2f8f4e';
    context.fillRect(0, 0, 64, 64);
    context.clearRect(24, 24, 16, 16);
  }, 64, 64);
  const scenery = new THREE.Mesh(new THREE.PlaneGeometry(2.2, 2.2), cutoutMaterial(leaves, 0xffffff, false));
  scenery.position.set(0, 0, 0.8);
  scene.add(scenery);
  const cast = new THREE.Mesh(new THREE.PlaneGeometry(1.4, 1.4), new THREE.MeshBasicMaterial({ color: 0xe23b2f }));
  cast.position.set(0, 0, 0);
  cast.userData.tokenId = 'ada';
  scene.add(cast);
  const outlines = new OcclusionOutlines();
  renderer.render(scene, camera);
  outlines.draw(renderer, scene, camera, [cast], null, [cast]);
  const pixels = new Uint8Array(css * css * 4);
  const gl = renderer.getContext();
  gl.readPixels(0, 0, css, css, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  let holeRed = 0;
  let holeCream = 0;
  let edgeCream = 0;
  for (let y = 0; y < css; y += 1) {
    for (let x = 0; x < css; x += 1) {
      const index = (y * css + x) * 4;
      const r = pixels[index] ?? 0;
      const g = pixels[index + 1] ?? 0;
      const b = pixels[index + 2] ?? 0;
      const worldX = (x / css) * 4 - 2;
      const worldY = (y / css) * 4 - 2;
      const inHole = Math.abs(worldX) < 0.12 && Math.abs(worldY) < 0.12;
      if (inHole && r > 160 && g < 100) holeRed += 1;
      if (inHole && isCream(r, g, b)) holeCream += 1;
      if (worldX < -0.55 && worldX > -0.85 && Math.abs(worldY) < 0.4 && isCream(r, g, b)) edgeCream += 1;
    }
  }
  outlines.dispose();
  renderer.dispose();
  if (holeRed < 8) throw new Error(`the hole in the scenery hid the figure (${holeRed} red pixels)`);
  if (holeCream > 0) throw new Error(`the hole drew a line across the figure (${holeCream} cream pixels)`);
  if (edgeCream < 8) throw new Error(`the covered edge beside the hole was not outlined (${edgeCream} cream pixels)`);
}

function isGold(r: number, g: number, b: number) {
  return r > 220 && g > 180 && b < 160 && b > 80;
}

function isPlainCream(r: number, g: number, b: number) {
  return r > 180 && g > 180 && b > 180;
}

async function outlinePass(ratio: number) {
  const css = 160;
  const buffer = css * ratio;
  const canvas = document.querySelector('#view') as HTMLCanvasElement;
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: false });
  renderer.setPixelRatio(ratio);
  renderer.setSize(css, css, false);
  renderer.setClearColor(0x100e0c, 1);
  const scene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-2, 2, 2, -2, 0.1, 20);
  camera.position.set(0, 0, 5);
  camera.lookAt(0, 0, 0);

  const scenery = new THREE.Mesh(
    new THREE.PlaneGeometry(2.2, 2.6),
    new THREE.MeshBasicMaterial({ color: 0x2f8f4e }),
  );
  scenery.position.set(-0.7, 0, 0.8);
  scene.add(scenery);

  const castTexture = canvasTexture((context) => {
    context.clearRect(0, 0, 64, 64);
    context.fillStyle = '#e23b2f';
    context.fillRect(0, 0, 64, 64);
    context.clearRect(24, 24, 16, 16);
  }, 64, 64);
  const cast = new THREE.Mesh(
    new THREE.PlaneGeometry(2, 2),
    new THREE.MeshBasicMaterial({ map: castTexture, transparent: true, alphaTest: 0.5 }),
  );
  cast.position.set(0, 0, 0);
  cast.userData.tokenId = 'ada';
  scene.add(cast);

  const outlines = new OcclusionOutlines();
  renderer.render(scene, camera);
  outlines.draw(renderer, scene, camera, [cast], null, [cast]);
  const pixels = new Uint8Array(buffer * buffer * 4);
  const gl = renderer.getContext();
  gl.readPixels(0, 0, buffer, buffer, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  let creamLeft = 0;
  let creamRight = 0;
  let creamCut = 0;
  let redRight = 0;
  const mid = buffer * 0.5;
  const visible = buffer * 0.625;
  for (let y = 0; y < buffer; y += 1) {
    for (let x = 0; x < buffer; x += 1) {
      const index = (y * buffer + x) * 4;
      const r = pixels[index] ?? 0;
      const g = pixels[index + 1] ?? 0;
      const b = pixels[index + 2] ?? 0;
      const worldX = (x / buffer) * 4 - 2;
      const worldY = (y / buffer) * 4 - 2;
      if (!isCream(r, g, b)) {
        if (x > visible && r > 180 && g < 90 && b < 90) redRight += 1;
        continue;
      }
      if (x < mid) creamLeft += 1;
      if (x > visible) creamRight += 1;
      const hole = Math.abs(worldX) < 0.35 && Math.abs(worldY) < 0.35;
      if (!hole && worldX > 0.32 && worldX < 0.55 && Math.abs(worldY) < 0.7) creamCut += 1;
    }
  }
  outlines.dispose();
  renderer.dispose();
  if (creamLeft === 0) throw new Error(`no outline where scenery is in front of the cast at pixel ratio ${ratio}`);
  if (creamRight > 0) throw new Error(`outline leaked onto the visible side at pixel ratio ${ratio} (${creamRight} pixels)`);
  if (creamCut > 0) throw new Error(`the occluder drew a line across the body at pixel ratio ${ratio} (${creamCut} pixels)`);
  if (redRight === 0) throw new Error(`the visible side of the cast did not draw at pixel ratio ${ratio}`);
}

function overlapPass() {
  const buffer = 160;
  const canvas = document.querySelector('#view') as HTMLCanvasElement;
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: false });
  renderer.setPixelRatio(1);
  renderer.setSize(buffer, buffer, false);
  renderer.setClearColor(0x100e0c, 1);
  const scene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-2, 2, 2, -2, 0.1, 20);
  camera.position.set(0, 0, 5);
  camera.lookAt(0, 0, 0);
  const scenery = new THREE.Mesh(new THREE.PlaneGeometry(2.2, 2.6), new THREE.MeshBasicMaterial({ color: 0x2f8f4e }));
  scenery.position.set(-0.7, 0, 0.8);
  scene.add(scenery);
  const solid = (color: string) => canvasTexture((context) => {
    context.fillStyle = color;
    context.fillRect(0, 0, 64, 64);
  }, 64, 64);
  const figure = (id: string, x: number, z: number, color: string) => {
    const mesh = new THREE.Mesh(
      new THREE.PlaneGeometry(1.4, 1.6),
      new THREE.MeshBasicMaterial({ map: solid(color), transparent: true, alphaTest: 0.5 }),
    );
    mesh.position.set(x, 0, z);
    mesh.userData.tokenId = id;
    scene.add(mesh);
    return mesh;
  };
  const ada = figure('ada', -0.45, 0, '#e23b2f');
  const bram = figure('bram', 0.45, 0.2, '#3b5ee2');
  const outlines = new OcclusionOutlines();
  renderer.render(scene, camera);
  outlines.draw(renderer, scene, camera, [ada, bram], 'ada', [ada, bram]);
  const pixels = new Uint8Array(buffer * buffer * 4);
  const gl = renderer.getContext();
  gl.readPixels(0, 0, buffer, buffer, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  let gold = 0;
  let cream = 0;
  for (let index = 0; index < pixels.length; index += 4) {
    const r = pixels[index] ?? 0;
    const g = pixels[index + 1] ?? 0;
    const b = pixels[index + 2] ?? 0;
    if (isGold(r, g, b)) gold += 1;
    else if (isPlainCream(r, g, b)) cream += 1;
  }
  outlines.dispose();
  renderer.dispose();
  if (gold < 8) throw new Error(`the selected figure did not get its own outline (${gold} gold pixels)`);
  if (cream < 8) throw new Error(`the other figure was fused into the selected outline (${cream} cream pixels)`);
}

function png(color: string, transparentTop: boolean) {
  const canvas = document.createElement('canvas');
  canvas.width = 32;
  canvas.height = 64;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('no canvas');
  context.clearRect(0, 0, 32, 64);
  context.fillStyle = color;
  context.fillRect(0, transparentTop ? 40 : 0, 32, transparentTop ? 24 : 64);
  return canvas.toDataURL('image/png');
}

function token(id: string, name: string, x: number, y: number, color: string): TokenView {
  return {
    id,
    name,
    x,
    y,
    kind: 'disc',
    imageUrl: null,
    color,
    ownerId: null,
    hidden: false,
    visionRadius: 6,
    grantsVision: true,
    characterId: null,
    castAsset: 'gate/hero',
    facing: 'south',
  };
}

function view(tokens: TokenView[], size: number): View {
  const layout: MapLayout = {
    width: size,
    height: size,
    terrains: ['gate/floor'],
    cells: btoa('\0'.repeat(size * size)),
    pieces: [],
    props: [],
    decals: [],
    lights: [],
    seed: 1,
  };
  const scene: SceneView = {
    id: 'gate',
    name: 'Gate',
    width: size,
    height: size,
    imageUrl: null,
    walls: [],
    lights: [],
    layout,
    presentation: null,
    schemaVersion: 2,
    visualProfile: 'dimetric-30-v1',
  };
  const fog: FogView = { cellsPerSquare: 1, width: size, height: size, explored: '', visiblePolygons: [], mode: 'gm' };
  const hero = png('#d4a656', true);
  const frame = { file: hero, sizePx: [32, 64] as [number, number], pivotPx: [16, 60] as [number, number] };
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
    contentHash: 'occlusion',
    tiles: [{ id: 'floor', name: 'Floor', kind: 'ground', sourceSizePx: [128, 128], coverageCells: [1, 1], files: [png('#c46a45', false)], transitionPolicy: 'rigid' }],
    cast: [{ id: 'hero', name: 'Hero', pixelsPerWorldUnit: 64, views: { north: frame, east: frame, south: frame, west: frame } }],
  };
  return {
    scene,
    tokens,
    doors: [],
    fog,
    role: 'gm',
    selectedId: null,
    targetId: null,
    hoverId: null,
    tool: 'select',
    packs: [pack],
    ghost: [],
    placeSpan: null,
    buildMode: false,
    viewId: 'gate',
    cast: {},
  };
}

async function tablePass() {
  const host = document.querySelector('#host') as HTMLElement;
  const engine = new TableEngine(host, {
    onMove: () => {},
    onSelect: () => {},
    onBrush: () => {},
    onToggleDoor: () => {},
    canDrag: () => false,
  });
  engine.start();
  try {
    const ada = token('ada', 'Ada', 4.5, 4.5, '#d4a656');
    const lurker = token('lurker', 'Lurker', 8.5, 8.5, '#2244ff');
    engine.sync(view([ada, lurker], 12));
    engine.centerOn(4.5, 4.5);
    await wait(250);
    const feet = engine.projectClient(4.5, 0.05, 4.5);
    const hair = engine.projectClient(4.5, 0.7, 4.5);
    const lurkerPoint = engine.projectClient(8.5, 0.4, 8.5);
    if (!feet || !hair || !lurkerPoint) throw new Error('a figure on a 12-square map was clipped');
    let feetHit: string | null = null;
    let hairHit: string | null = 'ada';
    for (let attempt = 0; attempt < 40; attempt += 1) {
      await wait(50);
      feetHit = engine.tokenAtPoint(feet.x, feet.y);
      hairHit = engine.tokenAtPoint(hair.x, hair.y);
      if (feetHit === 'ada' && hairHit !== 'ada') break;
    }
    if (feetHit !== 'ada') throw new Error(`the opaque feet returned ${feetHit ?? 'nothing'}`);
    if (hairHit === 'ada') throw new Error('a transparent pixel selected the figure');
    engine.sync(view([ada], 12));
    await wait(100);
    if (engine.tokenAtPoint(lurkerPoint.x, lurkerPoint.y) === 'lurker') throw new Error('a removed figure could still be selected');
    const dark = view([ada], 12);
    dark.role = 'player';
    dark.seatId = 'ada';
    dark.fog = {
      cellsPerSquare: 1,
      width: 12,
      height: 12,
      explored: '',
      visiblePolygons: [[[0, 0], [3, 0], [3, 3], [0, 3]]],
      mode: 'player',
    };
    engine.sync(dark);
    engine.centerOn(4.5, 4.5);
    await wait(80);
    if (engine.screenOf('ada')) throw new Error('a figure outside sight was drawn');
    ada.ownerId = 'ada';
    engine.sync(dark);
    await wait(80);
    if (!engine.screenOf('ada')) throw new Error('your own figure disappeared outside the light');
    const still = view([ada], 12);
    engine.sync(still);
    engine.renderNow();
    const before = engine.sceneryDraws;
    ada.x = 6.5;
    engine.sync(still);
    engine.renderNow();
    if (engine.sceneryDraws !== before) throw new Error(`moving a figure redrew the ground (${engine.sceneryDraws - before} extra)`);
    engine.centerOn(1, 1);
    engine.renderNow();
    if (engine.sceneryDraws === before) throw new Error('panning did not refresh the ground behind the figures');
    if (engine.projectClient(8.5, 0.8, 8.5) && engine.tokenAtPoint(lurkerPoint.x, lurkerPoint.y) === 'lurker') {
      throw new Error('the hidden figure stayed on the table');
    }

    engine.sync(view([token('west', 'West', 0.5, 0.5, '#d4a656'), token('east', 'East', 199.5, 199.5, '#7eb8a2')], 200));
    await wait(200);
    engine.centerOn(0.5, 0.5);
    await wait(50);
    if (!engine.projectClient(199.5, 4, 199.5)) throw new Error('the far corner of a 200-square map is behind the camera');
    engine.centerOn(199.5, 199.5);
    await wait(50);
    if (!engine.projectClient(0.5, 4, 0.5)) throw new Error('the opposite corner of a 200-square map is behind the camera');
  } finally {
    engine.dispose();
  }
}

const oakUrl = new URL('../../../../assets/packs/ruin/objects/oak.png', import.meta.url).href;
const wallUrl = new URL('../../../../assets/packs/ruin/objects/wall-h.png', import.meta.url).href;
const wallVUrl = new URL('../../../../assets/packs/ruin/objects/wall-v.png', import.meta.url).href;
const banditUrl = new URL('../../../../assets/packs/ruin/cast/bandit/south.png', import.meta.url).href;
const grassUrl = new URL('../../../../assets/packs/ruin/tiles/grass.png', import.meta.url).href;

function upright(file: string, sizePx: [number, number], pivotPx: [number, number], extra?: Partial<ObjectDef['renderLayers'][number]>) {
  return { kind: 'upright', file, sizePx, pivotPx, ...extra };
}

async function ruinOutline() {
  const host = document.querySelector('#host') as HTMLElement;
  host.style.width = '480px';
  host.style.height = '360px';
  const rootX = 4;
  const rootZ = 9;
  const wrenX = 4.52;
  const wrenZ = 8.48;
  const shadeX = 3.5;
  const shadeZ = 6.5;
  const oakResponse = await fetch(oakUrl);
  if (!oakResponse.ok) throw new Error(`the oak picture did not load (${oakResponse.status} ${oakUrl})`);
  let cream = 0;
  let green = 0;
  const spot = { x: wrenX, z: wrenZ, radius: 28 };
  const engine = new TableEngine(host, {
    onMove: () => {},
    onSelect: () => {},
    onBrush: () => {},
    onToggleDoor: () => {},
    canDrag: () => false,
    onPaint: () => {
      const sample = sampleNear(engine, spot.x, spot.z, spot.radius);
      cream = Math.max(cream, sample.cream);
      green = Math.max(green, sample.green);
    },
  });
  const frame = { file: banditUrl, sizePx: [32, 64] as [number, number], pivotPx: [16, 60] as [number, number] };
  const layout: MapLayout = {
    width: 12,
    height: 12,
    terrains: ['ruin/grass'],
    cells: btoa('\0'.repeat(144)),
    pieces: [
      { id: 'wh1', kind: 'wall', asset: 'ruin/wall', x: 1, y: 5, open: false, axis: 'h' },
      { id: 'wh2', kind: 'wall', asset: 'ruin/wall', x: 3, y: 5, open: false, axis: 'h' },
      { id: 'wv1', kind: 'wall', asset: 'ruin/wall', x: 4, y: 5, open: false, axis: 'v' },
      { id: 'wv2', kind: 'wall', asset: 'ruin/wall', x: 4, y: 6, open: false, axis: 'v' },
      { id: 'wv3', kind: 'wall', asset: 'ruin/wall', x: 4, y: 7, open: false, axis: 'v' },
    ],
    props: [{ id: 'oak', asset: 'ruin/oak', x: 3, y: 8, mirror: false }],
    decals: [],
    lights: [],
    seed: 7,
  };
  const scene: SceneView = {
    id: 'ruin',
    name: 'Ruin',
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
  const pack: Pack = {
    id: 'ruin',
    name: 'Ruin',
    pixelsPerCell: 128,
    filtering: 'nearest',
    terrains: [],
    decals: [],
    walls: [],
    doors: [],
    props: [],
    themes: [],
    baseUrl: '',
    contentHash: 'ruin-outline',
    tiles: [{ id: 'grass', name: 'Grass', kind: 'ground', sourceSizePx: [128, 128], coverageCells: [1, 1], files: [grassUrl], transitionPolicy: 'rigid' }],
    objects: [
      {
        id: 'oak',
        name: 'Oak',
        kind: 'static',
        placement: 'cell',
        footprintCells: [2, 2],
        originCells: [1, 1],
        pixelsPerWorldUnit: 128,
        occludesVisually: true,
        renderLayers: [upright(oakUrl, [64, 96], [32, 90])],
      },
      {
        id: 'wall',
        name: 'Wall',
        kind: 'wall',
        placement: 'edge',
        footprintCells: [1, 1],
        pixelsPerWorldUnit: 128,
        occludesVisually: true,
        renderLayers: [upright(wallUrl, [16, 48], [8, 46], { axis: 'h' }), upright(wallVUrl, [16, 48], [8, 46], { axis: 'v' })],
      },
    ],
    cast: [{ id: 'bandit', name: 'Bandit', pixelsPerWorldUnit: 128, views: { north: frame, east: frame, south: frame, west: frame } }],
  };
  const wren = token('wren', 'Wren', wrenX, wrenZ, '#d4a656');
  wren.castAsset = 'ruin/bandit';
  engine.start();
  try {
    engine.sync({
      scene,
      tokens: [wren],
      doors: [],
      fog: null,
      role: 'player',
      selectedId: null,
      targetId: null,
      hoverId: null,
      tool: 'select',
      packs: [pack],
      ghost: [],
      placeSpan: null,
      buildMode: false,
      viewId: 'ruin',
      cast: {},
    });
    engine.centerOn(rootX, rootZ);
    engine.zoomBy(0.2);
    engine.zoomBy(0.2);
    for (let attempt = 0; attempt < 30 && (cream < 8 || green < 20); attempt += 1) {
      engine.renderNow();
      await wait(100);
    }
    if (green < 20) throw new Error(`the oak picture is not on the table (${green} green pixels, ${oakUrl})`);
    if (cream < 8) throw new Error(`the oak did not outline the figure behind it (${cream} cream pixels)`);
    const shade = engine.projectClient(shadeX, 0.4, shadeZ);
    if (shade && engine.tokenAtPoint(shade.x, shade.y)) throw new Error('Shade, left out of the view, could still be hovered');
    const body = engine.projectClient(wrenX, 0.35, wrenZ);
    if (!body || engine.tokenAtPoint(body.x, body.y) !== 'wren') throw new Error('the permitted figure behind the oak could not be hovered');
    const lurker = token('lurker', 'Lurker', wrenX, wrenZ, '#6b5b4b');
    lurker.hidden = true;
    lurker.castAsset = 'ruin/bandit';
    engine.sync({
      scene,
      tokens: [wren, lurker],
      doors: [],
      fog: null,
      role: 'player',
      selectedId: null,
      targetId: null,
      hoverId: null,
      tool: 'select',
      packs: [pack],
      ghost: [],
      placeSpan: null,
      buildMode: false,
      viewId: 'ruin',
      cast: {},
    });
    await wait(100);
    if (engine.screenOf('lurker')) throw new Error('a hidden figure was drawn for the player');
    const after = engine.projectClient(wrenX, 0.35, wrenZ);
    if (!after || engine.tokenAtPoint(after.x, after.y) !== 'wren') throw new Error('the hidden figure could be hovered through Wren');
    const frontX = 2.2;
    const frontZ = 11;
    spot.x = frontX;
    spot.z = frontZ;
    spot.radius = 8;
    cream = 0;
    green = 0;
    const front = token('wren', 'Wren', frontX, frontZ, '#d4a656');
    front.castAsset = 'ruin/bandit';
    engine.sync({
      scene,
      tokens: [front],
      doors: [],
      fog: null,
      role: 'player',
      selectedId: null,
      targetId: null,
      hoverId: null,
      tool: 'select',
      packs: [pack],
      ghost: [],
      placeSpan: null,
      buildMode: false,
      viewId: 'ruin',
      cast: {},
    });
    for (let attempt = 0; attempt < 20; attempt += 1) {
      engine.renderNow();
      await wait(50);
    }
    if (cream >= 8) throw new Error(`the outline stayed after Wren walked in front of the oak (${cream} cream pixels)`);
    const frontHit = engine.projectClient(frontX, 0.35, frontZ);
    if (!frontHit || engine.tokenAtPoint(frontHit.x, frontHit.y) !== 'wren') throw new Error('Wren could not be hovered in front of the oak');
  } finally {
    engine.dispose();
  }
}

function isGreen(r: number, g: number, b: number) {
  return g > 70 && g > r + 25 && g > b + 25;
}

function isBlue(r: number, g: number, b: number) {
  return b > 120 && b > r + 30 && b > g;
}

function sampleNear(engine: TableEngine, x: number, z: number, radius = 28, y = 0.35) {
  const empty = { cream: 0, green: 0, blue: 0 };
  const canvas = document.querySelector('#host canvas') as HTMLCanvasElement | null;
  const point = engine.projectClient(x, y, z);
  if (!canvas || !point) return empty;
  const gl = canvas.getContext('webgl2');
  if (!gl) return empty;
  const rect = canvas.getBoundingClientRect();
  if (rect.width < 1 || rect.height < 1) return empty;
  const scaleX = gl.drawingBufferWidth / rect.width;
  const scaleY = gl.drawingBufferHeight / rect.height;
  const cx = Math.round((point.x - rect.left) * scaleX);
  const cy = Math.round((rect.bottom - point.y) * scaleY);
  const x0 = Math.max(0, cx - radius);
  const y0 = Math.max(0, cy - radius);
  const x1 = Math.min(gl.drawingBufferWidth, cx + radius);
  const y1 = Math.min(gl.drawingBufferHeight, cy + radius);
  const width = x1 - x0;
  const height = y1 - y0;
  if (width < 1 || height < 1) return empty;
  const pixels = new Uint8Array(width * height * 4);
  gl.readPixels(x0, y0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
  let cream = 0;
  let green = 0;
  let blue = 0;
  for (let index = 0; index < pixels.length; index += 4) {
    const r = pixels[index] ?? 0;
    const g = pixels[index + 1] ?? 0;
    const b = pixels[index + 2] ?? 0;
    if (isCream(r, g, b)) cream += 1;
    if (isGreen(r, g, b)) green += 1;
    if (isBlue(r, g, b)) blue += 1;
  }
  return { cream, green, blue };
}

const barrelUrl = new URL('../../../../assets/packs/ruin/objects/barrel.png', import.meta.url).href;

async function tierOutline() {
  const host = document.querySelector('#host') as HTMLElement;
  host.style.width = '480px';
  host.style.height = '360px';
  const figureX = 5.2;
  const figureZ = 2.8;
  let cream = 0;
  const spot = { x: figureX, y: 0.25, z: figureZ, radius: 18 };
  const engine = new TableEngine(host, {
    onMove: () => {},
    onSelect: () => {},
    onBrush: () => {},
    onToggleDoor: () => {},
    canDrag: () => false,
    onPaint: () => {
      cream = Math.max(cream, sampleNear(engine, spot.x, spot.z, spot.radius, spot.y).cream);
    },
  });
  const frame = { file: banditUrl, sizePx: [32, 64] as [number, number], pivotPx: [16, 60] as [number, number] };
  const pack: Pack = {
    id: 'ruin',
    name: 'Ruin',
    pixelsPerCell: 128,
    filtering: 'nearest',
    terrains: [],
    decals: [],
    walls: [],
    doors: [],
    props: [],
    themes: [],
    baseUrl: '',
    contentHash: 'tier-outline',
    tiles: [{ id: 'grass', name: 'Grass', kind: 'ground', sourceSizePx: [128, 128], coverageCells: [1, 1], files: [grassUrl], transitionPolicy: 'rigid' }],
    objects: [
      {
        id: 'barrel',
        name: 'Barrel',
        kind: 'static',
        placement: 'cell',
        footprintCells: [1, 1],
        pixelsPerWorldUnit: 128,
        occludesVisually: true,
        renderLayers: [upright(barrelUrl, [32, 48], [16, 46])],
      },
    ],
    cast: [{ id: 'bandit', name: 'Bandit', pixelsPerWorldUnit: 128, views: { north: frame, east: frame, south: frame, west: frame } }],
  };
  const sceneFor = (): SceneView => {
    const layout: MapLayout = {
      width: 8,
      height: 8,
      terrains: ['ruin/grass'],
      cells: btoa('\0'.repeat(64)),
      pieces: [],
      props: [{ id: 'barrel', asset: 'ruin/barrel', x: 4, y: 3, mirror: false }],
      decals: [],
      lights: [],
      seed: 3,
    };
    return {
      id: 'rise',
      name: 'Rise',
      width: 8,
      height: 8,
      imageUrl: null,
      walls: [],
      lights: [],
      layout,
      presentation: null,
      schemaVersion: 2,
      visualProfile: 'dimetric-30-v1',
    };
  };
  const show = (pictured: boolean) => {
    const figure = token('ada', 'Ada', figureX, figureZ, '#3a6ea5');
    if (pictured) figure.castAsset = 'ruin/bandit';
    else delete figure.castAsset;
    engine.sync({
      scene: sceneFor(),
      tokens: [figure],
      doors: [],
      fog: null,
      role: 'player',
      selectedId: null,
      targetId: null,
      hoverId: null,
      tool: 'select',
      packs: [pack],
      ghost: [],
      placeSpan: null,
      buildMode: false,
      viewId: 'rise',
      cast: {},
    });
  };
  const watch = async (pictured: boolean, bodyY: number) => {
    spot.y = bodyY;
    cream = 0;
    show(pictured);
    engine.fitMap();
    engine.centerOn(figureX, figureZ);
    spot.radius = 18;
    for (let attempt = 0; attempt < 24 && cream < 8; attempt += 1) {
      engine.renderNow();
      await wait(50);
    }
    return cream;
  };
  engine.start();
  try {
    const floor = await watch(true, 0.25);
    const bare = await watch(false, 0.25);
    if (floor < 8 || bare < 8) {
      const wide = sampleNear(engine, figureX, figureZ, 48, 0.4).cream;
      const drawn = engine.screenOf('ada') ? 'drawn' : 'missing';
      throw new Error(`barrel outline was floor ${floor}, no picture ${bare}, wide ${wide}, ${drawn}`);
    }
  } finally {
    engine.dispose();
  }
}

function tallBodyUrl() {
  const canvas = document.createElement('canvas');
  canvas.width = 32;
  canvas.height = 128;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('no canvas');
  context.clearRect(0, 0, 32, 128);
  context.fillStyle = '#3a6ea5';
  context.fillRect(6, 4, 20, 120);
  return canvas.toDataURL('image/png');
}

async function cliffOutline() {
  const host = document.querySelector('#host') as HTMLElement;
  host.style.width = '480px';
  host.style.height = '360px';
  const figureX = 3.5;
  const figureZ = 3.35;
  let lowCream = 0;
  let headCream = 0;
  let headBlue = 0;
  const engine = new TableEngine(host, {
    onMove: () => {},
    onSelect: () => {},
    onBrush: () => {},
    onToggleDoor: () => {},
    canDrag: () => false,
    onPaint: () => {
      lowCream = Math.max(lowCream, sampleNear(engine, figureX, figureZ, 20, 1.2).cream);
      const head = sampleNear(engine, figureX, figureZ, 10, 3.6);
      headCream = head.cream;
      headBlue = Math.max(headBlue, head.blue);
    },
  });
  const body = tallBodyUrl();
  const frame = { file: body, sizePx: [32, 128] as [number, number], pivotPx: [16, 120] as [number, number] };
  const levels = new Uint8Array(64);
  levels[4 * 8 + 2] = 2;
  levels[4 * 8 + 3] = 2;
  levels[4 * 8 + 4] = 2;
  const pack: Pack = {
    id: 'ruin',
    name: 'Ruin',
    pixelsPerCell: 128,
    filtering: 'nearest',
    terrains: [],
    decals: [],
    walls: [],
    doors: [],
    props: [],
    themes: [],
    baseUrl: '',
    contentHash: 'cliff-outline',
    tiles: [{ id: 'grass', name: 'Grass', kind: 'ground', sourceSizePx: [128, 128], coverageCells: [1, 1], files: [grassUrl], transitionPolicy: 'rigid' }],
    objects: [],
    cast: [{ id: 'tall', name: 'Tall', pixelsPerWorldUnit: 32, views: { north: frame, east: frame, south: frame, west: frame } }],
  };
  const layout: MapLayout = {
    width: 8,
    height: 8,
    terrains: ['ruin/grass'],
    cells: btoa('\0'.repeat(64)),
    levels: encodeCells(levels),
    pieces: [],
    props: [],
    decals: [],
    lights: [],
    seed: 4,
  };
  const figure = token('ada', 'Ada', figureX, figureZ, '#3a6ea5');
  figure.castAsset = 'ruin/tall';
  engine.sync({
    scene: {
      id: 'cliff',
      name: 'Cliff',
      width: 8,
      height: 8,
      imageUrl: null,
      walls: [],
      lights: [],
      layout,
      presentation: null,
      schemaVersion: 2,
      visualProfile: 'dimetric-30-v1',
    },
    tokens: [figure],
    doors: [],
    fog: null,
    role: 'player',
    selectedId: null,
    targetId: null,
    hoverId: null,
    tool: 'select',
    packs: [pack],
    ghost: [],
    placeSpan: null,
    buildMode: false,
    viewId: 'cliff',
    cast: {},
  });
  engine.start();
  try {
    engine.fitMap();
    engine.centerOn(figureX, figureZ);
    engine.zoomBy(0.45);
    for (let attempt = 0; attempt < 24 && (lowCream < 4 || headBlue < 8); attempt += 1) {
      engine.renderNow();
      await wait(50);
    }
    if (lowCream < 4 || headBlue < 8 || headCream > 0) {
      throw new Error(`cliff outline was low ${lowCream}, head cream ${headCream}, head blue ${headBlue}`);
    }
  } finally {
    engine.dispose();
  }
}

try {
  await outlinePass(1);
  await outlinePass(2);
  tierPass();
  holePass();
  overlapPass();
  say('OCCLUSION_STEP outline');
  await tablePass();
  say('OCCLUSION_STEP table');
  await ruinOutline();
  say('OCCLUSION_STEP ruin');
  await tierOutline();
  say('OCCLUSION_STEP cliff');
  await cliffOutline();
  say('OCCLUSION_OK');
} catch (error) {
  say(`OCCLUSION_FAIL ${error instanceof Error ? error.message : String(error)}`);
}
