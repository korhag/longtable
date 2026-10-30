// ------------------------------------------------------------
// Module: terrain
// Purpose: The ground of a built map. One source picture covers
//          one cell. A variant comes from the map seed, unless the
//          square keeps a picture, and a
//          rigid floor is not warped. The ground is split into
//          chunks so a height edit rebuilds the nearby squares.
//          A raised edge uses the pack's side picture.
// ------------------------------------------------------------

import * as THREE from 'three';

import { cachedImage, fileUrl, findIn, loadTexture, type Pack } from '../assets/catalog';
import { decodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { cellLevel, FOG_LIFT } from './levels';
import { pictureIndex } from './tileVariant';

const TILE = 128;
const CHUNK = 64;
export const terrainStats = { rebuilds: 0 };

type GroundChunk = {
  id: string;
  mesh: THREE.Mesh;
  sides: THREE.Mesh[];
};

/** Chunk ids covering a map. A 20-square map is one chunk. A 200-square map is sixteen. */
export function chunkIds(width: number, height: number): string[] {
  const ids: string[] = [];
  const cols = Math.max(1, Math.ceil(width / CHUNK));
  const rows = Math.max(1, Math.ceil(height / CHUNK));
  for (let row = 0; row < rows; row += 1) {
    for (let col = 0; col < cols; col += 1) ids.push(`${col},${row}`);
  }
  return ids;
}

/** Chunks whose top or side face changes when a square's height changes, plus the neighbor that shares the edge. */
export function dirtyChunkIds(width: number, height: number, previous: Uint8Array | null, next: Uint8Array | null): string[] {
  const ids = new Set<string>();
  const mark = (x: number, y: number) => {
    if (x < 0 || y < 0 || x >= width || y >= height) return;
    ids.add(`${Math.floor(x / CHUNK)},${Math.floor(y / CHUNK)}`);
  };
  const count = width * height;
  for (let index = 0; index < count; index += 1) {
    if ((previous?.[index] ?? 0) === (next?.[index] ?? 0)) continue;
    const x = index % width;
    const y = Math.floor(index / width);
    mark(x, y);
    mark(x + 1, y);
    mark(x - 1, y);
    mark(x, y + 1);
    mark(x, y - 1);
  }
  return [...ids];
}

const VERT = `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

const FRAG = `
  uniform sampler2D cells;
  uniform sampler2DArray tiles;
  uniform vec2 mapSize;
  varying vec2 vUv;
  out vec4 fragColor;

  float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
  }

  vec2 lookup(vec2 cellPos) {
    vec2 p = (floor(cellPos) + 0.5) / mapSize;
    return vec2(p.x, 1.0 - p.y);
  }

  void main() {
    vec2 cell = vec2(vUv.x, 1.0 - vUv.y) * mapSize;
    vec4 centerSample = texture(cells, lookup(cell));
    float index = centerSample.r * 255.0;
    if (centerSample.a > 0.5) {
      vec2 nudged = cell + vec2(hash(floor(cell)) - 0.5, hash(floor(cell) + 19.0) - 0.5);
      vec4 other = texture(cells, lookup(clamp(nudged, vec2(0.001), mapSize - 0.001)));
      float pick = hash(floor(cell) + 4.0);
      float centerLayer = centerSample.g * 255.0;
      float otherLayer = other.g * 255.0;
      float threshold = otherLayer > centerLayer + 0.5 ? 0.35 : 0.72;
      index = pick > threshold ? other.r * 255.0 : index;
    }
    vec2 tileUv = fract(cell);
    fragColor = texture(tiles, vec3(tileUv, index));
  }
`;

type Slice = {
  url: string | null;
  swatch: string;
  filtering: Pack['filtering'];
};

export class TerrainLayer {
  private chunks: GroundChunk[] = [];
  private material: THREE.ShaderMaterial | null = null;
  private ramps: THREE.Mesh | null = null;
  private sliceKey = '';
  private shape = '';
  private heights: Uint8Array | null = null;
  private generation = 0;
  private onPaint: () => void = () => {};

  constructor(private scene: THREE.Scene) {}

  sync(layout: MapLayout | null, packs: Pack[], onPaint: () => void = () => {}) {
    this.onPaint = onPaint;
    if (!layout || layout.terrains.length === 0) {
      this.clear();
      return;
    }
    const built = buildGround(layout, packs);
    const slices = built.slices.map((slice) => `${slice.url ?? ''}|${slice.swatch}|${slice.filtering}`).join(';');
    const shape = `${layout.levels ?? ''}|${(layout.stairs ?? []).map((stair) => `${stair.x},${stair.y},${stair.x2},${stair.y2}`).join(';')}`;
    const reusable = this.cellImage(layout);
    if (reusable && slices === this.sliceKey && shape === this.shape) {
      writeCells(reusable.data, built);
      reusable.texture.needsUpdate = true;
      const size = this.material?.uniforms.mapSize?.value as THREE.Vector2 | undefined;
      size?.set(layout.width, layout.height);
      return;
    }
    const heights = heightBytes(layout);
    const sameAtlas = Boolean(reusable && slices === this.sliceKey);
    let painted: { data: Uint8Array; tiles: THREE.DataArrayTexture } | null = null;
    if (!sameAtlas || !reusable) {
      this.clear();
      this.generation += 1;
      painted = this.installMaterial(layout, built);
      this.sliceKey = slices;
    } else {
      writeCells(reusable.data, built);
      reusable.texture.needsUpdate = true;
      const size = this.material?.uniforms.mapSize?.value as THREE.Vector2 | undefined;
      size?.set(layout.width, layout.height);
    }
    const ids = sameAtlas ? dirtyChunkIds(layout.width, layout.height, this.heights, heights) : chunkIds(layout.width, layout.height);
    const generation = this.generation;
    for (const id of ids) {
      this.rebuildChunk(id, layout, packs, heights, generation);
      terrainStats.rebuilds += 1;
    }
    this.replaceRamps(layout);
    this.shape = shape;
    this.heights = heights ? new Uint8Array(heights) : null;
    if (painted) void paintFiles(painted.data, painted.tiles, built.slices, generation, () => this.generation, () => this.onPaint());
  }

  hit(raycaster: THREE.Raycaster): { point: THREE.Vector3; stair: boolean } | null {
    const targets = [...this.chunks.map((chunk) => chunk.mesh), this.ramps].filter((mesh): mesh is THREE.Mesh => mesh !== null);
    if (targets.length === 0) return null;
    const hit = raycaster.intersectObjects(targets, false)[0];
    if (!hit) return null;
    return { point: hit.point.clone(), stair: hit.object.userData.kind === 'stair' };
  }

  clear() {
    this.generation += 1;
    this.shape = '';
    this.heights = null;
    for (const chunk of this.chunks) this.disposeChunk(chunk);
    this.chunks = [];
    if (this.ramps) {
      this.ramps.geometry.dispose();
      (this.ramps.material as THREE.Material).dispose();
      this.ramps.removeFromParent();
      this.ramps = null;
    }
    if (this.material) {
      (this.material.uniforms.cells?.value as THREE.Texture | undefined)?.dispose();
      (this.material.uniforms.tiles?.value as THREE.Texture | undefined)?.dispose();
      this.material.dispose();
      this.material = null;
    }
    this.sliceKey = '';
  }

  private cellImage(layout: MapLayout): { data: Uint8Array; texture: THREE.DataTexture } | null {
    const cells = this.material?.uniforms.cells?.value as THREE.DataTexture | undefined;
    const image = cells?.image as { width?: number; height?: number; data?: Uint8Array } | undefined;
    if (!cells || !image?.data || image.width !== layout.width || image.height !== layout.height) return null;
    return { data: image.data, texture: cells };
  }

  private installMaterial(layout: MapLayout, built: ReturnType<typeof buildGround>) {
    const data = new Uint8Array(TILE * TILE * 4 * built.slices.length);
    for (let layer = 0; layer < built.slices.length; layer += 1) fill(data, layer, built.slices[layer]?.swatch ?? '#6e675f');
    const tiles = new THREE.DataArrayTexture(data, TILE, TILE, built.slices.length);
    tiles.colorSpace = THREE.SRGBColorSpace;
    tiles.format = THREE.RGBAFormat;
    tiles.wrapS = THREE.ClampToEdgeWrapping;
    tiles.wrapT = THREE.ClampToEdgeWrapping;
    const nearest = built.slices.some((slice) => slice.filtering === 'nearest');
    tiles.magFilter = nearest ? THREE.NearestFilter : THREE.LinearFilter;
    tiles.minFilter = nearest ? THREE.NearestFilter : THREE.LinearFilter;
    tiles.needsUpdate = true;
    this.material = new THREE.ShaderMaterial({
      glslVersion: THREE.GLSL3,
      uniforms: {
        cells: { value: cellTexture(layout, built) },
        tiles: { value: tiles },
        mapSize: { value: new THREE.Vector2(layout.width, layout.height) },
      },
      vertexShader: VERT,
      fragmentShader: FRAG,
    });
    return { data, tiles };
  }

  private rebuildChunk(id: string, layout: MapLayout, packs: Pack[], heights: Uint8Array | null, generation: number) {
    this.dropChunk(id);
    if (!this.material) return;
    const [col, row] = id.split(',').map((part) => Number(part));
    if (!Number.isInteger(col) || !Number.isInteger(row)) return;
    const x0 = col * CHUNK;
    const y0 = row * CHUNK;
    const x1 = Math.min(layout.width, x0 + CHUNK);
    const y1 = Math.min(layout.height, y0 + CHUNK);
    const mesh = new THREE.Mesh(chunkGeometry(layout.width, layout.height, x0, y0, x1, y1, heights), this.material);
    mesh.position.y = 0.004;
    mesh.renderOrder = 0;
    this.scene.add(mesh);
    const sides: THREE.Mesh[] = [];
    for (const batch of sideBatches(layout, packs, heights, { x0, y0, x1, y1 })) {
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(batch.positions, 3));
      geometry.setAttribute('uv', new THREE.Float32BufferAttribute(batch.uvs, 2));
      const material = new THREE.MeshBasicMaterial({ color: 0x6a6158, side: THREE.DoubleSide });
      const side = new THREE.Mesh(geometry, material);
      side.raycast = () => {};
      side.renderOrder = 0;
      side.userData.kind = 'side';
      side.userData.url = batch.url ?? '';
      this.scene.add(side);
      sides.push(side);
      if (!batch.url) continue;
      void loadTexture(batch.url, batch.filtering).then((texture) => {
        if (!texture || this.generation !== generation || !side.parent) return;
        texture.wrapS = THREE.RepeatWrapping;
        texture.wrapT = THREE.RepeatWrapping;
        texture.needsUpdate = true;
        material.map = texture;
        material.color.set(0xffffff);
        material.needsUpdate = true;
        this.onPaint();
      });
    }
    this.chunks.push({ id, mesh, sides });
  }

  private dropChunk(id: string) {
    const index = this.chunks.findIndex((chunk) => chunk.id === id);
    if (index < 0) return;
    const chunk = this.chunks[index];
    if (!chunk) return;
    this.disposeChunk(chunk);
    this.chunks.splice(index, 1);
  }

  private disposeChunk(chunk: GroundChunk) {
    chunk.mesh.geometry.dispose();
    chunk.mesh.removeFromParent();
    for (const side of chunk.sides) {
      side.geometry.dispose();
      const material = side.material as THREE.MeshBasicMaterial;
      material.map = null;
      material.dispose();
      side.removeFromParent();
    }
  }

  private replaceRamps(layout: MapLayout) {
    if (this.ramps) {
      this.ramps.geometry.dispose();
      (this.ramps.material as THREE.Material).dispose();
      this.ramps.removeFromParent();
      this.ramps = null;
    }
    const slopes = ramps(layout);
    if (!slopes) return;
    this.ramps = new THREE.Mesh(slopes, new THREE.MeshBasicMaterial({ color: 0xc4b49a, side: THREE.DoubleSide }));
    this.ramps.userData.kind = 'stair';
    this.ramps.renderOrder = 1;
    this.scene.add(this.ramps);
  }
}

function buildGround(layout: MapLayout, packs: Pack[]) {
  const bytes = decodeCells(layout.cells);
  const defs = layout.terrains.map((asset) => groundDef(packs, asset));
  const slices: Slice[] = [];
  const sliceOf = new Map<string, number>();
  const chosen = new Uint8Array(layout.width * layout.height);
  const organic = new Uint8Array(layout.width * layout.height);
  const layers = new Uint8Array(layout.width * layout.height);
  const seed = layout.seed ?? 1;
  const pins = variantBytes(layout);
  for (let index = 0; index < chosen.length; index += 1) {
    const terrain = Math.min(bytes[index] ?? 0, Math.max(0, defs.length - 1));
    const def = defs[terrain];
    if (!def) continue;
    const variant = pictureIndex(seed, index % layout.width, Math.floor(index / layout.width), def.id, def.files.length, pins[index] ?? 0);
    const key = `${terrain}:${variant}`;
    let slice = sliceOf.get(key);
    if (slice === undefined) {
      slice = slices.length;
      sliceOf.set(key, slice);
      slices.push({ url: def.files[variant] ?? null, swatch: def.swatch, filtering: def.filtering });
    }
    chosen[index] = slice;
    organic[index] = def.organic ? 255 : 0;
    layers[index] = def.layer;
  }
  if (slices.length === 0) slices.push({ url: null, swatch: '#6e675f', filtering: 'linear' });
  return { slices, chosen, organic, layers };
}

function variantBytes(layout: MapLayout): Uint8Array {
  const count = layout.width * layout.height;
  if (!layout.variants) return new Uint8Array(count);
  try {
    const decoded = decodeCells(layout.variants);
    if (decoded.length === count) return decoded;
  } catch {
    // A broken picture grid falls back to the seed for every square.
  }
  return new Uint8Array(count);
}

function groundDef(packs: Pack[], asset: string) {
  const tile = findIn(packs, asset, (pack) => pack.tiles ?? []);
  if (tile && (tile.def.kind === 'ground' || tile.def.kind === 'side')) {
    return {
      id: asset,
      files: tile.def.files.length > 0 ? tile.def.files.map((file) => fileUrl(tile.pack, file)) : [null],
      swatch: '#6e675f',
      organic: tile.def.transitionPolicy === 'organic',
      layer: tile.def.kind === 'ground' ? 1 : 0,
      filtering: tile.pack.filtering,
    };
  }
  const terrain = findIn(packs, asset, (pack) => pack.terrains);
  const files = terrain?.def.files?.filter((file) => file.length > 0) ?? [];
  return {
    id: asset,
    files: files.length > 0 ? files.map((file) => fileUrl(terrain!.pack, file)) : [null],
    swatch: terrain?.def.swatch ?? '#6e675f',
    organic: false,
    layer: terrain?.def.layer ?? 0,
    filtering: terrain?.pack.filtering ?? 'linear',
  };
}

const GRID_LIFT = 0.012;

/** Grid lines sit on the higher square beside each edge, so a rise does not leave the floor grid inside the block. */
export function gridPositions(mapWidth: number, mapHeight: number, layout: MapLayout | null): number[] {
  const columns = Math.min(Math.ceil(mapWidth), 160);
  const rows = Math.min(Math.ceil(mapHeight), 160);
  const levels = layout ? heightBytes(layout) : null;
  const span = Math.max(1, Math.ceil(mapWidth));
  const tierAt = (x: number, z: number) => {
    if (!levels || x < 0 || z < 0 || x >= span || z >= Math.ceil(mapHeight)) return 0;
    return Math.min(2, levels[z * span + x] ?? 0);
  };
  const points: number[] = [];
  for (let z = 0; z <= rows; z += 1) {
    for (let x = 0; x < columns; x += 1) {
      const y = Math.max(tierAt(x, z - 1), tierAt(x, z)) + GRID_LIFT;
      points.push(x, y, z, x + 1, y, z);
    }
  }
  for (let x = 0; x <= columns; x += 1) {
    for (let z = 0; z < rows; z += 1) {
      const y = Math.max(tierAt(x - 1, z), tierAt(x, z)) + GRID_LIFT;
      points.push(x, y, z, x, y, z + 1);
    }
  }
  return points;
}

function heightBytes(layout: MapLayout): Uint8Array | null {
  if (!layout.levels) return null;
  try {
    const bytes = decodeCells(layout.levels);
    if (bytes.length !== layout.width * layout.height) return null;
    if (!bytes.some((value) => value > 0)) return null;
    return bytes;
  } catch {
    return null;
  }
}

function chunkGeometry(width: number, height: number, x0: number, y0: number, x1: number, y1: number, levels: Uint8Array | null) {
  let tier = 0;
  let uniform = true;
  if (levels) {
    tier = Math.min(2, levels[y0 * width + x0] ?? 0);
    for (let y = y0; y < y1 && uniform; y += 1) {
      for (let x = x0; x < x1; x += 1) {
        if (Math.min(2, levels[y * width + x] ?? 0) !== tier) uniform = false;
      }
    }
  }
  const geometry = new THREE.BufferGeometry();
  if (!levels || uniform) {
    geometry.setAttribute('position', new THREE.Float32BufferAttribute([x0, tier, y0, x1, tier, y0, x1, tier, y1, x0, tier, y1], 3));
    geometry.setAttribute('uv', new THREE.Float32BufferAttribute([
      x0 / width, 1 - y0 / height,
      x1 / width, 1 - y0 / height,
      x1 / width, 1 - y1 / height,
      x0 / width, 1 - y1 / height,
    ], 2));
    geometry.setIndex([0, 2, 1, 0, 3, 2]);
    return geometry;
  }
  const positions: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  for (let y = y0; y < y1; y += 1) {
    for (let x = x0; x < x1; x += 1) {
      const cell = Math.min(2, levels[y * width + x] ?? 0);
      const base = positions.length / 3;
      positions.push(x, cell, y, x + 1, cell, y, x + 1, cell, y + 1, x, cell, y + 1);
      uvs.push(x / width, 1 - y / height, (x + 1) / width, 1 - y / height, (x + 1) / width, 1 - (y + 1) / height, x / width, 1 - (y + 1) / height);
      indices.push(base, base + 2, base + 1, base, base + 3, base + 2);
    }
  }
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
  geometry.setIndex(indices);
  return geometry;
}

function flatGround(width: number, height: number) {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute([0, 0, 0, width, 0, 0, width, 0, height, 0, 0, height], 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute([0, 1, 1, 1, 1, 0, 0, 0], 2));
  geometry.setIndex([0, 2, 1, 0, 3, 2]);
  return geometry;
}

export type SideBatch = {
  url: string | null;
  filtering: Pack['filtering'];
  positions: number[];
  uvs: number[];
};

export function sideBatches(
  layout: MapLayout,
  packs: Pack[],
  levels: Uint8Array | null,
  range?: { x0: number; y0: number; x1: number; y1: number },
): SideBatch[] {
  if (!levels) return [];
  const width = layout.width;
  const height = layout.height;
  const batches = new Map<string, SideBatch>();
  const owns = (x: number, y: number) => !range || (x >= range.x0 && x < range.x1 && y >= range.y0 && y < range.y1);
  const push = (x0: number, z0: number, x1: number, z1: number, low: number, high: number, ownerX: number, ownerY: number) => {
    if (high <= low || !owns(ownerX, ownerY)) return;
    const source = sideSource(packs, terrainAt(layout, ownerX, ownerY));
    const key = source?.url ?? '';
    const batch = batches.get(key) ?? { url: source?.url ?? null, filtering: source?.filtering ?? 'linear', positions: [], uvs: [] };
    const span = high - low;
    batch.positions.push(x0, low, z0, x1, low, z1, x1, high, z1, x0, low, z0, x1, high, z1, x0, high, z0);
    batch.uvs.push(0, 0, 1, 0, 1, span, 0, 0, 1, span, 0, span);
    batches.set(key, batch);
  };
  const tier = (x: number, y: number) => (x < 0 || y < 0 || x >= width || y >= height ? 0 : Math.min(2, levels[y * width + x] ?? 0));
  const yStart = range ? range.y0 - 1 : 0;
  const xStart = range ? range.x0 - 1 : 0;
  const yEnd = range ? range.y1 : height;
  const xEnd = range ? range.x1 : width;
  for (let y = yStart; y < yEnd; y += 1) {
    for (let x = xStart; x < xEnd; x += 1) {
      if (x < 0 || y < 0 || x >= width || y >= height) continue;
      const here = tier(x, y);
      const east = tier(x + 1, y);
      const south = tier(x, y + 1);
      const eastOwner = here >= east || x + 1 >= width ? { x, y } : { x: x + 1, y };
      const southOwner = here >= south || y + 1 >= height ? { x, y } : { x, y: y + 1 };
      push(x + 1, y, x + 1, y + 1, Math.min(here, east), Math.max(here, east), eastOwner.x, eastOwner.y);
      push(x, y + 1, x + 1, y + 1, Math.min(here, south), Math.max(here, south), southOwner.x, southOwner.y);
      if (x === 0) push(x, y + 1, x, y, 0, here, x, y);
      if (y === 0) push(x + 1, y, x, y, 0, here, x, y);
    }
  }
  return [...batches.values()];
}

function terrainAt(layout: MapLayout, x: number, y: number) {
  let index = 0;
  try {
    const bytes = decodeCells(layout.cells);
    index = Math.min(bytes[y * layout.width + x] ?? 0, Math.max(0, layout.terrains.length - 1));
  } catch {
    index = 0;
  }
  return layout.terrains[index] ?? '';
}

function sideSource(packs: Pack[], asset: string) {
  const tile = findIn(packs, asset, (pack) => pack.tiles ?? []);
  if (!tile) return null;
  const painted = tile.def.kind === 'side' ? tile.def : null;
  const fallback = tile.pack.tiles?.find((item) => item.kind === 'side' && item.files.length > 0) ?? null;
  const chosen = painted?.files.length ? painted : fallback;
  const file = chosen?.files[0];
  if (!file) return null;
  return { url: fileUrl(tile.pack, file), filtering: tile.pack.filtering };
}

function ramps(layout: MapLayout) {
  const stairs = layout.stairs ?? [];
  if (stairs.length === 0) return null;
  const positions: number[] = [];
  for (const stair of stairs) {
    const low = cellLevel(layout, stair.x, stair.y);
    const high = cellLevel(layout, stair.x2, stair.y2);
    if (Math.abs(low - high) !== 1) continue;
    const from = low < high ? { x: stair.x, y: stair.y, tier: low } : { x: stair.x2, y: stair.y2, tier: high };
    const to = low < high ? { x: stair.x2, y: stair.y2, tier: high } : { x: stair.x, y: stair.y, tier: low };
    const dx = to.x - from.x;
    const dz = to.y - from.y;
    const px = -dz * 0.35;
    const pz = dx * 0.35;
    const ax = from.x + 0.5;
    const az = from.y + 0.5;
    const bx = to.x + 0.5;
    const bz = to.y + 0.5;
    positions.push(ax + px, from.tier + 0.02, az + pz, ax - px, from.tier + 0.02, az - pz, bx - px, to.tier + 0.02, bz - pz);
    positions.push(ax + px, from.tier + 0.02, az + pz, bx - px, to.tier + 0.02, bz - pz, bx + px, to.tier + 0.02, bz + pz);
  }
  if (positions.length === 0) return null;
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  return geometry;
}

function fill(data: Uint8Array, layer: number, swatch: string) {
  const rgb = hex(swatch);
  const start = layer * TILE * TILE * 4;
  for (let pixel = 0; pixel < TILE * TILE; pixel += 1) {
    const offset = start + pixel * 4;
    data[offset] = rgb[0];
    data[offset + 1] = rgb[1];
    data[offset + 2] = rgb[2];
    data[offset + 3] = 255;
  }
}

function writeCells(image: Uint8Array, built: ReturnType<typeof buildGround>) {
  for (let index = 0; index < built.chosen.length; index += 1) {
    image[index * 4] = built.chosen[index] ?? 0;
    image[index * 4 + 1] = built.layers[index] ?? 0;
    image[index * 4 + 3] = built.organic[index] ?? 0;
  }
}

function cellTexture(layout: MapLayout, built: ReturnType<typeof buildGround>) {
  const image = new Uint8Array(layout.width * layout.height * 4);
  writeCells(image, built);
  const texture = new THREE.DataTexture(image, layout.width, layout.height, THREE.RGBAFormat);
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestFilter;
  texture.colorSpace = THREE.NoColorSpace;
  texture.needsUpdate = true;
  return texture;
}

async function paintFiles(data: Uint8Array, tiles: THREE.DataArrayTexture, slices: Slice[], generation: number, current: () => number, onPaint: () => void) {
  for (let layer = 0; layer < slices.length; layer += 1) {
    const url = slices[layer]?.url;
    if (!url) continue;
    const image = await cachedImage(url);
    if (!image || current() !== generation) return;
    const canvas = document.createElement('canvas');
    canvas.width = TILE;
    canvas.height = TILE;
    const context = canvas.getContext('2d');
    if (!context) continue;
    context.drawImage(image, 0, 0, TILE, TILE);
    const pixels = context.getImageData(0, 0, TILE, TILE).data;
    data.set(pixels, layer * TILE * TILE * 4);
    tiles.needsUpdate = true;
    onPaint();
  }
}

export function fogShell(width: number, height: number, layout: MapLayout | null): { geometry: THREE.BufferGeometry; lifted: boolean } {
  const levels = layout ? heightBytes(layout) : null;
  if (!levels) {
    return { geometry: flatGround(width, height), lifted: false };
  }
  const positions: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  const uv = (x: number, z: number): [number, number] => [x / width, 1 - z / height];
  const addQuad = (corners: Array<[number, number, number]>, sample: (x: number, z: number) => [number, number]) => {
    const base = positions.length / 3;
    for (const [x, y, z] of corners) {
      positions.push(x, y, z);
      uvs.push(...sample(x, z));
    }
    indices.push(base, base + 2, base + 1, base, base + 3, base + 2);
  };
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const tier = Math.min(2, levels[y * width + x] ?? 0) + FOG_LIFT;
      addQuad(
        [
          [x, tier, y],
          [x + 1, tier, y],
          [x + 1, tier, y + 1],
          [x, tier, y + 1],
        ],
        uv,
      );
      const east = x + 1 >= width ? 0 : Math.min(2, levels[y * width + x + 1] ?? 0);
      const south = y + 1 >= height ? 0 : Math.min(2, levels[(y + 1) * width + x] ?? 0);
      const here = tier - FOG_LIFT;
      if (here !== east) {
        const low = Math.min(here, east) + FOG_LIFT;
        const high = Math.max(here, east) + FOG_LIFT;
        const intoHigher = here > east ? -0.25 : 0.25;
        addQuad(
          [
            [x + 1, low, y],
            [x + 1, low, y + 1],
            [x + 1, high, y + 1],
            [x + 1, high, y],
          ],
          (px, pz) => uv(px + intoHigher, pz),
        );
      }
      if (here !== south) {
        const low = Math.min(here, south) + FOG_LIFT;
        const high = Math.max(here, south) + FOG_LIFT;
        const intoHigher = here > south ? -0.25 : 0.25;
        addQuad(
          [
            [x, low, y + 1],
            [x + 1, low, y + 1],
            [x + 1, high, y + 1],
            [x, high, y + 1],
          ],
          (px, pz) => uv(px, pz + intoHigher),
        );
      }
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
  geometry.setIndex(indices);
  return { geometry, lifted: true };
}

function hex(color: string): [number, number, number] {
  const value = Number.parseInt(color.replace('#', ''), 16);
  if (!Number.isFinite(value)) return [110, 103, 95];
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255];
}
