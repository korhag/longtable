import * as THREE from 'three';
import type { CastAnimations } from '../assets/catalog';
import type { CastAnimationKind } from './castAnimation';
import { AtlasTextureCache, type AtlasLease } from './atlasTextureCache';
import { cutoutPlacement } from './cutout';

/** Frame UVs live on the figure geometry, never on the shared texture. Picking
 * and the silhouette pass consequently sample the same drawn frame as the body.
 */
export function atlasUvs(base: Float32Array, output: Float32Array, definition: CastAnimations, row: number, column: number) {
  const [width, height] = definition.frameSizePx;
  const atlasWidth = width * definition.columns, atlasHeight = height * definition.rows;
  for (let i = 0; i < base.length; i += 2) {
    output[i] = (column * width + .5 + base[i]! * (width - 1)) / atlasWidth;
    output[i + 1] = ((definition.rows - row - 1) * height + .5 + base[i + 1]! * (height - 1)) / atlasHeight;
  }
}

export class CastAtlas {
  private geometry: THREE.BufferGeometry | null = null;
  private positions: Float32Array = new Float32Array();
  private uvs = new Float32Array();
  private atlasPositions = new Float32Array();
  private baseTexture: THREE.Texture | null = null;
  private atlasTexture: THREE.Texture | null = null;
  private definition: CastAnimations | null = null;
  private source: { url: string; filtering: 'linear' | 'nearest'; onPaint: () => void } | null = null;
  private lease: AtlasLease | null = null;
  private frame = -1;
  private active = false;
  private generation = 0;
  private retryAt = 0;

  constructor(private card: THREE.Mesh, private cache: AtlasTextureCache) {}

  setGeometry(geometry: THREE.BufferGeometry, rest: Float32Array) {
    this.detach();
    this.geometry = geometry;
    this.positions = rest;
    this.uvs = new Float32Array(geometry.getAttribute('uv').array);
    this.atlasPositions = new Float32Array(rest.length);
    this.baseTexture = null;
  }

  setBaseTexture(texture: THREE.Texture) {
    this.baseTexture = texture;
    if (!this.active) this.material.map = texture;
  }

  configure(definition: CastAnimations | undefined, url: string | null, filtering: 'linear' | 'nearest', onPaint: () => void): Promise<unknown> {
    this.detach();
    if (!definition || !url || !this.geometry) return Promise.resolve();
    this.definition = definition;
    this.source = { url, filtering, onPaint };
    const place = cutoutPlacement(definition.frameSizePx, definition.pivotPx, definition.pixelsPerWorldUnit);
    let radius = 0;
    for (let i = 0; i < this.positions.length; i += 3) {
      const vertex = i / 3;
      const x = (this.uvs[vertex * 2]! - .5) * place.width + place.offsetX;
      const y = (this.uvs[vertex * 2 + 1]! - .5) * place.height + place.offsetY;
      this.atlasPositions[i] = x; this.atlasPositions[i + 1] = y; this.atlasPositions[i + 2] = 0;
      radius = Math.max(radius, Math.hypot(x, y), Math.hypot(this.positions[i]!, this.positions[i + 1]!));
    }
    this.geometry.boundingSphere = new THREE.Sphere(new THREE.Vector3(), radius + .1);
    this.geometry.boundingBox = null;
    return this.load();
  }

  private get material() { return this.card.material as THREE.MeshBasicMaterial; }
  get configured() { return this.definition !== null; }

  duration(kind: CastAnimationKind) {
    const clip = this.definition?.clips[kind];
    return clip ? clip.frames / clip.fps * 1000 * (kind === 'idle' ? 2 : kind === 'move' ? 3 : 1) : null;
  }

  private load(): Promise<unknown> {
    if (!this.source || this.lease) return this.lease?.ready ?? Promise.resolve();
    const source = this.source, generation = this.generation;
    this.lease = this.cache.acquire(source.url, source.filtering);
    if (!this.lease) return Promise.resolve();
    return this.lease.ready.then(texture => {
      if (generation !== this.generation) return;
      this.atlasTexture = texture;
      if (texture && !this.active) this.rest();
      if (!texture) {
        this.lease?.release(); this.lease = null;
        this.retryAt = performance.now() + 3000;
      }
      source.onPaint();
    });
  }

  /** Returns only the next real frame boundary; no extra animation RAF. */
  show(kind: CastAnimationKind, elapsed: number, now: number): number {
    if (!this.definition || !this.geometry) return Infinity;
    if (!this.atlasTexture) {
      if (!this.lease && now >= this.retryAt) { this.retryAt = now + 1000; void this.load(); }
      return 1000;
    }
    const clip = this.definition.clips[kind], step = 1000 / clip.fps;
    const sequence = Math.max(0, Math.floor(elapsed / step));
    const column = kind === 'idle' || kind === 'move' ? sequence % clip.frames : Math.min(clip.frames - 1, sequence);
    const frame = clip.row * this.definition.columns + column;
    if (!this.active) {
      const position = this.geometry.getAttribute('position');
      (position.array as Float32Array).set(this.atlasPositions); position.needsUpdate = true;
      this.material.map = this.atlasTexture;
      this.material.needsUpdate = true;
      this.active = true;
    }
    if (frame !== this.frame) {
      const uv = this.geometry.getAttribute('uv');
      atlasUvs(this.uvs, uv.array as Float32Array, this.definition, clip.row, column);
      uv.needsUpdate = true;
      this.frame = frame;
    }
    return step - elapsed % step;
  }

  /** Keep the same drawn character artwork between clips and for reduced motion. */
  rest() {
    if (this.atlasTexture && this.definition) this.show('idle', 0, 0);
    else this.restore();
  }

  restore() {
    if (this.active && this.geometry) {
      const position = this.geometry.getAttribute('position'), uv = this.geometry.getAttribute('uv');
      (position.array as Float32Array).set(this.positions); position.needsUpdate = true;
      (uv.array as Float32Array).set(this.uvs); uv.needsUpdate = true;
      this.material.map = this.baseTexture;
      this.material.needsUpdate = true;
    }
    this.frame = -1; this.active = false;
  }

  detach() {
    this.restore();
    this.generation++;
    this.lease?.release(); this.lease = null;
    this.atlasTexture = null; this.definition = null; this.source = null; this.retryAt = 0;
  }
}
