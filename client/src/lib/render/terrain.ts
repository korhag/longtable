// ------------------------------------------------------------
// Module: terrain
// Purpose: The ground of a built map. Each cell picks a terrain,
//          and noise lets a higher layer creep over a lower one.
//          A missing picture uses the terrain's flat swatch.
// ------------------------------------------------------------

import * as THREE from 'three';

import { fileUrl, findIn, type Pack } from '../assets/catalog';
import { decodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';

const TILE = 128;

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

  float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
  }

  float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    float a = hash(i);
    float b = hash(i + vec2(1.0, 0.0));
    float c = hash(i + vec2(0.0, 1.0));
    float d = hash(i + vec2(1.0, 1.0));
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
  }

  vec2 lookup(vec2 cellPos) {
    vec2 p = (floor(cellPos) + 0.5) / mapSize;
    return vec2(p.x, 1.0 - p.y);
  }

  void main() {
    vec2 cell = vec2(vUv.x, 1.0 - vUv.y) * mapSize;
    float n = noise(cell * 0.8);
    float m = noise(cell * 0.8 + 17.2);
    vec2 warped = clamp(cell + vec2(n - 0.5, m - 0.5) * 1.15, vec2(0.001), mapSize - 0.001);
    vec4 centerSample = texture(cells, lookup(cell));
    vec4 warpSample = texture(cells, lookup(warped));
    float centerIndex = centerSample.r * 255.0;
    float warpIndex = warpSample.r * 255.0;
    float centerLayer = centerSample.g * 255.0;
    float warpLayer = warpSample.g * 255.0;
    float pick = noise(cell * 1.7);
    float threshold = warpLayer > centerLayer + 0.5 ? 0.35 : 0.72;
    float index = pick > threshold ? warpIndex : centerIndex;
    vec2 tileUv = fract(warped / 4.0);
    gl_FragColor = texture(tiles, vec3(tileUv, index));
  }
`;

export class TerrainLayer {
  private mesh: THREE.Mesh | null = null;
  private key = '';
  private generation = 0;

  constructor(private scene: THREE.Scene) {}

  sync(layout: MapLayout | null, packs: Pack[]) {
    if (!layout || layout.terrains.length === 0) {
      this.clear();
      return;
    }
    const key = `${layout.width}x${layout.height}:${layout.cells}:${layout.terrains.join('|')}:${packs.map((pack) => pack.id).join()}`;
    if (key === this.key && this.mesh) return;
    this.key = key;
    this.clear();
    const depth = layout.terrains.length;
    const data = new Uint8Array(TILE * TILE * 4 * depth);
    for (let layer = 0; layer < depth; layer += 1) {
      const found = findIn(packs, layout.terrains[layer] ?? '', (pack) => pack.terrains);
      fill(data, layer, found?.def.swatch ?? '#6e675f');
    }
    const tiles = new THREE.DataArrayTexture(data, TILE, TILE, depth);
    tiles.colorSpace = THREE.SRGBColorSpace;
    tiles.format = THREE.RGBAFormat;
    tiles.wrapS = THREE.RepeatWrapping;
    tiles.wrapT = THREE.RepeatWrapping;
    tiles.magFilter = THREE.LinearFilter;
    tiles.minFilter = THREE.LinearFilter;
    tiles.needsUpdate = true;

    const cells = cellTexture(layout, packs);
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

    const generation = ++this.generation;
    void paintFiles(data, tiles, layout, packs, generation, () => this.generation);
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
  }
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

function cellTexture(layout: MapLayout, packs: Pack[]) {
  const bytes = decodeCells(layout.cells);
  const image = new Uint8Array(layout.width * layout.height * 4);
  for (let index = 0; index < layout.width * layout.height; index += 1) {
    const terrain = bytes[index] ?? 0;
    const found = findIn(packs, layout.terrains[terrain] ?? '', (pack) => pack.terrains);
    image[index * 4] = terrain;
    image[index * 4 + 1] = found?.def.layer ?? 0;
    image[index * 4 + 3] = 255;
  }
  const texture = new THREE.DataTexture(image, layout.width, layout.height, THREE.RGBAFormat);
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestFilter;
  texture.colorSpace = THREE.NoColorSpace;
  texture.needsUpdate = true;
  return texture;
}

async function paintFiles(data: Uint8Array, tiles: THREE.DataArrayTexture, layout: MapLayout, packs: Pack[], generation: number, current: () => number) {
  for (let layer = 0; layer < layout.terrains.length; layer += 1) {
    const found = findIn(packs, layout.terrains[layer] ?? '', (pack) => pack.terrains);
    const url = found ? fileUrl(found.pack, found.def.files?.[0]) : null;
    if (!url) continue;
    const image = await loadImage(url);
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
  }
}

function loadImage(url: string) {
  return new Promise<HTMLImageElement | null>((resolve) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => resolve(null);
    image.src = url;
  });
}

function hex(color: string): [number, number, number] {
  const value = Number.parseInt(color.replace('#', ''), 16);
  if (!Number.isFinite(value)) return [110, 103, 95];
  return [(value >> 16) & 255, (value >> 8) & 255, value & 255];
}
