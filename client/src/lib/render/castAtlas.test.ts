import * as THREE from 'three';
import { expect, it, vi } from 'vitest';
import { resolveAnimationView, type CastAnimations } from '../assets/catalog';
import { AtlasTextureCache } from './atlasTextureCache';
import { CastAnimator, ANIMATION_KINDS } from './castAnimation';
import { atlasUvs } from './castAtlas';
import { OcclusionOutlines } from './occlusion';

const definition: CastAnimations = {
  frameSizePx: [128, 128], pivotPx: [64, 120], pixelsPerWorldUnit: 64, columns: 4, rows: 6,
  views: { north: { file: 'north.png' }, east: { file: 'east.png' }, south: { file: 'south.png' }, west: { file: 'west.png' } },
  clips: { idle: { row: 0, frames: 4, fps: 4 }, melee: { row: 1, frames: 4, fps: 8 }, ranged: { row: 2, frames: 4, fps: 8 }, move: { row: 3, frames: 4, fps: 8 }, hurt: { row: 4, frames: 4, fps: 8 }, spell: { row: 5, frames: 4, fps: 8 } },
};

it('resolves per-direction foot pivots and raster scale without changing the shared defaults', () => {
  const changed: CastAnimations = { ...definition, views: { ...definition.views, east: { file: 'wide.png', pixelsPerWorldUnit: 32, pivotPx: [94, 120] } } };
  expect(resolveAnimationView(changed, 'east')).toMatchObject({ file: 'wide.png', definition: { pixelsPerWorldUnit: 32, pivotPx: [94, 120] } });
  expect(resolveAnimationView(changed, 'north')).toMatchObject({ file: 'north.png', definition: { pixelsPerWorldUnit: 64, pivotPx: [64, 120] } });
  expect(changed.pixelsPerWorldUnit).toBe(64);
  expect(changed.pivotPx).toEqual([64, 120]);
});
async function figure(cache: AtlasTextureCache, url = 'south.png') {
  const base = new THREE.Texture();
  const card = new THREE.Mesh(new THREE.PlaneGeometry(), new THREE.MeshBasicMaterial({ map: base }));
  const animator = new CastAnimator(card, cache);
  animator.setFrame([128, 256], [64, 240], 128, 'south', true);
  animator.setRestTexture(base);
  await animator.setAnimationAtlas(definition, url, 'nearest', () => {});
  return { base, card, animator };
}

it('addresses all 24 cells inside their texel borders with top-to-bottom rows', () => {
  const base = new Float32Array([0, 0, 1, 1]), output = new Float32Array(4);
  const cells = new Set<string>();
  for (let row = 0; row < 6; row++) for (let column = 0; column < 4; column++) {
    atlasUvs(base, output, definition, row, column);
    expect(output[0]).toBeGreaterThan(column / 4);
    expect(output[2]).toBeLessThan((column + 1) / 4);
    expect(output[1]).toBeGreaterThan((5 - row) / 6);
    expect(output[3]).toBeLessThan((6 - row) / 6);
    cells.add([...output].join(','));
  }
  expect(cells.size).toBe(24);
});

it('plays drawn frames without deforming them, then rests on the same artwork at idle frame zero', async () => {
  const atlas = new THREE.Texture();
  const cache = new AtlasTextureCache(12, async () => atlas);
  const { animator, card } = await figure(cache);
  const position = card.geometry.getAttribute('position') as THREE.BufferAttribute;
  expect(position.count).toBe(4);
  const uv = card.geometry.getAttribute('uv') as THREE.BufferAttribute;
  const rest = new Float32Array(position.array), restUv = new Float32Array(uv.array);
  for (const kind of ANIMATION_KINDS) {
    animator.play(kind, 0);
    animator.tick(1, false, true, false, false);
    const drawnPositions = new Float32Array(position.array), version = uv.version;
    expect(card.material.map).toBe(atlas);
    animator.tick(300, false, true, false, false);
    expect(uv.array).not.toEqual(restUv);
    expect(position.array).toEqual(drawnPositions);
    expect(uv.version).toBeGreaterThan(version);
    expect(card.children.every(child => !child.visible)).toBe(true);
    expect(animator.tick(animator.clipDuration(kind), false, true, false, false)).toBe(Infinity);
    expect(card.material.map).toBe(atlas);
    expect(position.array).toEqual(rest);
    expect(uv.array).toEqual(restUv);
  }
  animator.dispose(); cache.dispose();
});

it('shares a sheet without sharing frame state, and masks/picking receive the current UVs', async () => {
  const texture = new THREE.Texture(), loader = vi.fn(async () => texture);
  const cache = new AtlasTextureCache(12, loader);
  const first = await figure(cache), second = await figure(cache);
  const outlines = new OcclusionOutlines() as unknown as { castChanged: (bodies: THREE.Mesh[], stamp: number) => boolean; dispose: () => void };
  first.animator.play('melee', 0); second.animator.play('spell', 0);
  first.animator.tick(1, false, true, false, false); second.animator.tick(300, false, true, false, false);
  expect(loader).toHaveBeenCalledOnce();
  expect(first.card.material.map).toBe(second.card.material.map);
  expect(first.card.geometry.getAttribute('uv').array).not.toEqual(second.card.geometry.getAttribute('uv').array);
  expect(texture.offset.toArray()).toEqual([0, 0]);
  expect(texture.repeat.toArray()).toEqual([1, 1]);
  expect(outlines.castChanged([first.card], 1)).toBe(true);
  expect(outlines.castChanged([first.card], 1)).toBe(false);
  const ray = new THREE.Raycaster(new THREE.Vector3(0, 1, 2), new THREE.Vector3(0, 0, -1));
  const before = ray.intersectObject(first.card, false)[0]!.uv!.clone();
  first.animator.tick(140, false, true, false, false);
  const after = ray.intersectObject(first.card, false)[0]!.uv!;
  expect(after.x - before.x).toBeCloseTo(.25);
  expect(after.y).toBeCloseTo(before.y);
  expect(outlines.castChanged([first.card], 1)).toBe(true);
  expect(first.animator.tick(180, false, true, true, false)).toBe(Infinity);
  expect(first.card.material.map).toBe(texture);
  first.animator.dispose(); second.animator.dispose(); cache.dispose(); outlines.dispose();
});

it('does not install a late atlas when facing changes or the actor is removed', async () => {
  let finish: (texture: THREE.Texture) => void = () => {};
  const cache = new AtlasTextureCache(1, () => new Promise(resolve => { finish = resolve; }));
  const base = new THREE.Texture();
  const card = new THREE.Mesh(new THREE.PlaneGeometry(), new THREE.MeshBasicMaterial({ map: base }));
  const animator = new CastAnimator(card, cache);
  animator.setFrame([128, 256], [64, 240], 128, 'south'); animator.setRestTexture(base);
  const ready = animator.setAnimationAtlas(definition, 'late.png', 'linear', () => {});
  animator.dispose();
  finish(new THREE.Texture()); await ready;
  expect(card.material.map).toBe(base);
  expect(cache.inUse).toBe(0);
  cache.dispose();
});
