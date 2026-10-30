// ------------------------------------------------------------
// Module: terrain
// Purpose: The ground of a built map. One source picture covers
//          one cell. A variant comes from the map seed, and a
//          rigid floor is not warped.
// ------------------------------------------------------------

import * as THREE from 'three';

import { cachedImage, fileUrl, findIn, type Pack } from '../assets/catalog';
import { decodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { tileVariant } from './tileVariant';

const TILE = 128;
export const terrainStats = { rebuilds: 0 };

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
  private mesh: THREE.Mesh | null = null;
  private key = '';
  private sliceKey = '';
  private generation = 0;
  private onPaint: () => void = () => {};

  constructor(private scene: THREE.Scene) {}

  sync(layout: MapLayout | null, packs: Pack[], onPaint: () => void = () => {}) {
    this.onPaint = onPaint;
    if (!layout || layout.terrains.length === 0) {
      this.clear();
      return;
    }
    const key = groundKey(layout, packs);
    const built = buildGround(layout, packs);
    const slices = built.slices.map((slice) => `${slice.url ?? ''}|${slice.swatch}|${slice.filtering}`).join(';');
    if (this.mesh && slices === this.sliceKey) {
      const material = this.mesh.material as THREE.ShaderMaterial;
      const cells = material.uniforms.cells?.value as THREE.DataTexture | undefined;
      const image = cells?.image as { width?: number; height?: number; data?: Uint8Array } | undefined;
      if (cells && image?.data && image.width === layout.width && image.height === layout.height) {
        writeCells(image.data, built);
        cells.needsUpdate = true;
        const size = material.uniforms.mapSize?.value as THREE.Vector2 | undefined;
        size?.set(layout.width, layout.height);
        this.key = key;
        return;
      }
    }
    this.clear();
    this.sliceKey = slices;
    this.key = key;
    const data = new Uint8Array(TILE * TILE * 4 * built.slices.length);
    for (let layer = 0; layer < built.slices.length; layer += 1) {
      fill(data, layer, built.slices[layer]?.swatch ?? '#6e675f');
    }
    const tiles = new THREE.DataArrayTexture(data, TILE, TILE, built.slices.length);
    tiles.colorSpace = THREE.SRGBColorSpace;
    tiles.format = THREE.RGBAFormat;
    tiles.wrapS = THREE.ClampToEdgeWrapping;
    tiles.wrapT = THREE.ClampToEdgeWrapping;
    const nearest = built.slices.some((slice) => slice.filtering === 'nearest');
    tiles.magFilter = nearest ? THREE.NearestFilter : THREE.LinearFilter;
    tiles.minFilter = nearest ? THREE.NearestFilter : THREE.LinearFilter;
    tiles.needsUpdate = true;

    const cells = cellTexture(layout, built);
    const material = new THREE.ShaderMaterial({
      glslVersion: THREE.GLSL3,
      uniforms: {
        cells: { value: cells },
        tiles: { value: tiles },
        mapSize: { value: new THREE.Vector2(layout.width, layout.height) },
      },
      vertexShader: VERT,
      fragmentShader: FRAG,
    });
    const geometry = ground(layout.width, layout.height);
    this.mesh = new THREE.Mesh(geometry, material);
    this.mesh.position.y = 0.004;
    this.mesh.renderOrder = 0;
    this.scene.add(this.mesh);
    terrainStats.rebuilds += 1;

    const generation = ++this.generation;
    void paintFiles(data, tiles, built.slices, generation, () => this.generation, () => this.onPaint());
  }

  clear() {
    this.generation += 1;
    if (!this.mesh) return;
    const material = this.mesh.material as THREE.ShaderMaterial;
    (material.uniforms.cells?.value as THREE.Texture | undefined)?.dispose();
    (material.uniforms.tiles?.value as THREE.Texture | undefined)?.dispose();
    material.dispose();
    this.mesh.geometry.dispose();
    this.mesh.removeFromParent();
    this.mesh = null;
    this.sliceKey = '';
  }
}

function groundKey(layout: MapLayout, packs: Pack[]) {
  const files = packs.map((pack) => `${pack.id}:${pack.contentHash ?? ''}:${(pack.tiles ?? []).map((tile) => tile.files.join(',')).join(';')}`).join('|');
  return `${layout.width}x${layout.height}:${layout.seed ?? 1}:${layout.cells}:${layout.terrains.join(',')}:${files}`;
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
  for (let index = 0; index < chosen.length; index += 1) {
    const terrain = Math.min(bytes[index] ?? 0, Math.max(0, defs.length - 1));
    const def = defs[terrain];
    if (!def) continue;
    const variant = tileVariant(seed, index % layout.width, Math.floor(index / layout.width), def.id, def.files.length);
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

function ground(width: number, height: number) {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute([0, 0, 0, width, 0, 0, width, 0, height, 0, 0, height], 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute([0, 1, 1, 1, 1, 0, 0, 0], 2));
  geometry.setIndex([0, 2, 1, 0, 3, 2]);
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

function hex(color: string): [number, number, number] {
  const value = Number.parseInt(color.replace('#', ''), 16);
  if (!Number.isFinite(value)) return [110, 103, 95];
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255];
}
