// ------------------------------------------------------------
// Module: occlusionCheck
// Purpose: Draw the real outline pass and the real table camera.
//          A hidden figure must not be selectable, a transparent
//          sprite pixel must not be a hit, and a 200-square map
//          must keep its corners in front of the camera.
// ------------------------------------------------------------

import * as THREE from 'three';

import type { Pack } from '../assets/catalog';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { SceneView } from '../net/bindings/SceneView.ts';
import type { TokenView } from '../net/bindings/TokenView.ts';
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
  let redRight = 0;
  const mid = buffer * 0.5;
  const visible = buffer * 0.625;
  for (let y = 0; y < buffer; y += 1) {
    for (let x = 0; x < buffer; x += 1) {
      const index = (y * buffer + x) * 4;
      const r = pixels[index] ?? 0;
      const g = pixels[index + 1] ?? 0;
      const b = pixels[index + 2] ?? 0;
      if (!isCream(r, g, b)) {
        if (x > visible && r > 180 && g < 90 && b < 90) redRight += 1;
        continue;
      }
      if (x < mid) creamLeft += 1;
      if (x > visible) creamRight += 1;
    }
  }
  outlines.dispose();
  renderer.dispose();
  if (creamLeft === 0) throw new Error(`no outline where scenery is in front of the cast at pixel ratio ${ratio}`);
  if (creamRight > 0) throw new Error(`outline leaked onto the visible side at pixel ratio ${ratio} (${creamRight} pixels)`);
  if (redRight === 0) throw new Error(`the visible side of the cast did not draw at pixel ratio ${ratio}`);
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

try {
  await outlinePass(1);
  await outlinePass(2);
  say('OCCLUSION_STEP outline');
  await tablePass();
  say('OCCLUSION_OK');
} catch (error) {
  say(`OCCLUSION_FAIL ${error instanceof Error ? error.message : String(error)}`);
}
