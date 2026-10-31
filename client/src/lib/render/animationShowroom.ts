/** Standalone visual fixture: no app session, campaign mutations, or socket. */
import * as THREE from 'three';
import { animationFileUrl, fileUrl, loadCatalog, loadTexture, resolveAnimationView, type CastDef, type Pack } from '../assets/catalog';
import { ANIMATION_KINDS, CastAnimator, CLIP_MS, type CastAnimationKind, type CastFacing } from './castAnimation';
import { COS_ELEVATION } from './cameraProfile';
import { opaqueTexel } from './cutout';
import { AtlasTextureCache } from './atlasTextureCache';

const ids = ['human', 'elf', 'dwarf', 'orc', 'bandit-scout', 'bandit-archer', 'bandit-brute', 'bandit-captain'];
const facings: CastFacing[] = ['north', 'east', 'south', 'west'];
const params = new URLSearchParams(location.search);
let facing: CastFacing = facings.includes(params.get('facing') as CastFacing) ? params.get('facing') as CastFacing : 'south';
const canvas = document.querySelector<HTMLCanvasElement>('#view')!;
const status = document.querySelector<HTMLDivElement>('#status')!;
const renderer = new THREE.WebGLRenderer({ canvas, antialias: false, preserveDrawingBuffer: true });
renderer.setSize(1400, 1320, false);
renderer.setPixelRatio(1);
renderer.outputColorSpace = THREE.SRGBColorSpace;
const scene = new THREE.Scene(); scene.background = new THREE.Color('#15120f');
const camera = new THREE.OrthographicCamera(0, 21, 1, -18.8, .1, 100);
camera.position.set(0, 0, 20);
const figures: { card: THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>; animator: CastAnimator; kind: CastAnimationKind; asset: CastDef }[] = [];
const atlasCache = new AtlasTextureCache(12);
let pack: Pack;
let frameId = 0;

async function setFacing(next: CastFacing) {
  facing = next;
  await Promise.all(figures.map(async figure => {
    const frame = figure.asset.views[next];
    figure.animator.setFrame(frame.sizePx, frame.pivotPx, figure.asset.pixelsPerWorldUnit ?? 128, next, !!figure.asset.animations);
    const texture = await loadTexture(fileUrl(pack, frame.file)!, pack.filtering);
    if (texture) figure.animator.setRestTexture(texture);
    const sheet = resolveAnimationView(figure.asset.animations, next);
    await figure.animator.setAnimationAtlas(sheet?.definition, animationFileUrl(pack, sheet?.file), pack.filtering, () => {});
  }));
  document.querySelector<HTMLSelectElement>('#facing')!.value = facing;
}
function pose(progress = .42) {
  cancelAnimationFrame(frameId);
  for (const figure of figures) {
    figure.animator.play(figure.kind, 0);
    figure.animator.tick(figure.animator.clipDuration(figure.kind) * progress, false, false, false, false);
  }
  renderer.render(scene, camera);
}
function poseFrame(column: number) {
  cancelAnimationFrame(frameId);
  for (const figure of figures) {
    figure.animator.play(figure.kind, 0);
    const fps = figure.asset.animations?.clips[figure.kind].fps;
    const time = fps ? (column + .1) * 1000 / fps : CLIP_MS[figure.kind] * (column + .1) / 4;
    figure.animator.tick(time, false, false, false, false);
  }
  renderer.render(scene, camera);
}
function play() {
  cancelAnimationFrame(frameId);
  const started = performance.now();
  const duration = Math.max(...figures.map(figure => figure.animator.clipDuration(figure.kind)));
  for (const figure of figures) figure.animator.play(figure.kind, started);
  const frame = () => {
    if (document.hidden) return;
    const now = performance.now();
    for (const figure of figures) figure.animator.tick(now, false, false, false, false);
    renderer.render(scene, camera);
    if (now - started <= duration + 50) frameId = requestAnimationFrame(frame);
  };
  frame();
}

async function check() {
  let opaqueCards = 0, transparentRejected = 0;
  const ray = new THREE.Raycaster();
  const direction = new THREE.Vector3(0, 0, -1);
  const drawn = figures.every(figure => !!figure.asset.animations);
  for (const next of facings) {
    await setFacing(next);
    for (let column = 0; column < (drawn ? 4 : 1); column++) {
      if (drawn) poseFrame(column); else pose();
      scene.updateMatrixWorld(true);
      for (const figure of figures) {
        let found = false;
        for (let y = .35; y < 1.85 && !found; y += .22) for (let x = -.4; x <= .4; x += .13) {
          ray.set(new THREE.Vector3(figure.card.position.x + x, figure.card.position.y + y, 10), direction);
          const hit = ray.intersectObject(figure.card, false)[0];
          if (!hit?.uv) continue;
          const opaque = opaqueTexel(figure.card.material.map?.image as CanvasImageSource | undefined, hit.uv.x, hit.uv.y);
          if (opaque) { found = true; break; } else transparentRejected++;
        }
        if (found) opaqueCards++;
      }
    }
  }
  // Warm up the shot cue, which first becomes visible after the release frame.
  pose(.62); pose(1); pose(.62);
  const memory = { ...renderer.info.memory };
  for (let repeat = 0; repeat < 4; repeat++) { pose(.2); pose(.62); pose(1); }
  const stableMemory = JSON.stringify(memory) === JSON.stringify(renderer.info.memory);
  await setFacing(facing); pose();
  const result = { combinations: 192, drawnFrames: drawn ? 768 : 0, opaqueCards, transparentRejected, stableMemory, memory, restImages: 32, cachedAtlases: atlasCache.size, activeAtlases: atlasCache.inUse, glError: renderer.getContext().getError() };
  status.textContent = JSON.stringify(result);
  return result;
}

async function main() {
  pack = (await loadCatalog()).find(pack => pack.id === 'forest')!;
  if (!pack) throw new Error('Forest pack missing');
  await Promise.all(ids.flatMap(id => {
    const asset = (pack.cast ?? []).find(cast => cast.id === id)!;
    return facings.map(facing => loadTexture(fileUrl(pack, asset.views[facing].file)!, pack.filtering));
  }));
  for (let column = 0; column < ids.length; column++) {
    const asset = (pack.cast ?? []).find(cast => cast.id === ids[column])!;
    const name = document.createElement('div'); name.className = 'name'; name.textContent = asset.name;
    name.style.left = `${100 + column * 160}px`; document.querySelector('#stage')!.append(name);
    for (let row = 0; row < ANIMATION_KINDS.length; row++) {
      const card = new THREE.Mesh(new THREE.PlaneGeometry(), new THREE.MeshBasicMaterial({ alphaTest: .5, side: THREE.DoubleSide }));
      card.position.set(2.7 + column * 2.4, -2.6 - row * 3.1, 0);
      card.scale.y = COS_ELEVATION;
      scene.add(card);
      figures.push({ card, animator: new CastAnimator(card, atlasCache), kind: ANIMATION_KINDS[row]!, asset });
    }
  }
  for (let row = 0; row < ANIMATION_KINDS.length; row++) {
    const label = document.createElement('div'); label.className = 'state'; label.textContent = ANIMATION_KINDS[row]!;
    label.style.top = `${140 + row * 207}px`; document.querySelector('#stage')!.append(label);
  }
  await setFacing(facing); pose(Number(params.get('progress') ?? .42));
  document.querySelector<HTMLSelectElement>('#facing')!.onchange = async event => { await setFacing((event.target as HTMLSelectElement).value as CastFacing); pose(); };
  document.querySelector<HTMLButtonElement>('#play')!.onclick = play;
  document.querySelector<HTMLButtonElement>('#pose')!.onclick = () => pose();
  Object.assign(window, { animationShowroom: { setFacing, pose, poseFrame, play, check, renderer, figures, atlasCache } });
  status.textContent = `ANIMATION_SHOWROOM_READY · ${figures.every(figure => !!figure.asset.animations) ? '768 drawn frames in 32 directional sheets' : '32 original still images with fallback motion'} · 192 combinations · local previews only`;
  if (params.has('check')) await check();
}
void main().catch(error => { status.textContent = String(error); console.error(error); });
