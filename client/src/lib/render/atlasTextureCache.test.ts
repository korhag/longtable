import * as THREE from 'three';
import { expect, it, vi } from 'vitest';
import { AtlasTextureCache } from './atlasTextureCache';

it('shares atlas loads and bounds both live and pending entries', async () => {
  const loader = vi.fn(async () => new THREE.Texture());
  const cache = new AtlasTextureCache(2, loader);
  const first = cache.acquire('one', 'nearest')!;
  const shared = cache.acquire('one', 'nearest')!;
  const second = cache.acquire('two', 'nearest')!;
  expect(first.ready).toBe(shared.ready);
  expect(cache.acquire('three', 'nearest')).toBeNull();
  expect(loader).toHaveBeenCalledTimes(2);
  const texture = await first.ready;
  const dispose = vi.spyOn(texture!, 'dispose');
  first.release();
  expect(cache.acquire('three', 'nearest')).toBeNull();
  shared.release(); shared.release();
  const third = cache.acquire('three', 'nearest')!;
  expect(cache.size).toBe(2);
  expect(dispose).toHaveBeenCalledOnce();
  expect(cache.inUse).toBe(2);
  second.release(); third.release(); cache.dispose();
  expect(cache.size).toBe(0);
});

it('disposes a late atlas result after its renderer or unused lease was retired', async () => {
  let finish: (texture: THREE.Texture) => void = () => {};
  const cache = new AtlasTextureCache(1, () => new Promise(resolve => { finish = resolve; }));
  const lease = cache.acquire('late', 'linear')!;
  cache.dispose();
  const texture = new THREE.Texture();
  const dispose = vi.spyOn(texture, 'dispose');
  finish(texture);
  expect(await lease.ready).toBeNull();
  expect(dispose).toHaveBeenCalledOnce();
  lease.release();
  expect(cache.size).toBe(0);
});

it('does not replace an unused pending request and exceed the in-flight cap', async () => {
  const pending: ((texture: THREE.Texture) => void)[] = [];
  const loader = vi.fn(() => new Promise<THREE.Texture>(resolve => { pending.push(resolve); }));
  const cache = new AtlasTextureCache(12, loader);
  const leases = Array.from({ length: 12 }, (_, index) => cache.acquire(`${index}.png`, 'nearest')!);
  for (const lease of leases) lease.release();
  expect(cache.acquire('13th.png', 'nearest')).toBeNull();
  expect(loader).toHaveBeenCalledTimes(12);
  const texture = new THREE.Texture(), dispose = vi.spyOn(texture, 'dispose');
  pending[0]!(texture);
  await leases[0]!.ready;
  expect(cache.acquire('13th.png', 'nearest')).not.toBeNull();
  expect(loader).toHaveBeenCalledTimes(13);
  expect(dispose).toHaveBeenCalledOnce();
  cache.dispose();
  expect(cache.acquire('after-dispose.png', 'nearest')).toBeNull();
  for (const finish of pending.slice(1)) finish(new THREE.Texture());
});
