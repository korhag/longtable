// ------------------------------------------------------------
// Module: catalog
// Purpose: The asset packs the server loaded. Pictures are
//          fetched from /packs and cached as textures.
// ------------------------------------------------------------

import * as THREE from 'three';

export interface TerrainDef {
  id: string;
  name: string;
  swatch: string;
  layer: number;
  files?: string[];
}

export interface DecalDef {
  id: string;
  name: string;
  file?: string | null;
  size: [number, number];
  rotate?: boolean;
}

export interface WallDef {
  id: string;
  name: string;
  front?: string | null;
  side?: string | null;
  post?: string | null;
}

export interface DoorDef {
  id: string;
  name: string;
  wall: string;
  frontClosed?: string | null;
  frontOpen?: string | null;
  sideClosed?: string | null;
  sideOpen?: string | null;
}

export interface PropDef {
  id: string;
  name: string;
  file?: string | null;
  variants?: string[];
  footprint: [number, number];
  blocks: 'none' | 'move' | 'sight';
  anchor?: [number, number] | null;
  mirror?: boolean;
  height?: number;
}

export interface ScatterDef {
  asset: string;
  density: number;
  spacing: number;
}

export interface ThemeDef {
  id: string;
  name: string;
  terrain: string;
  wall?: string | null;
  door?: string | null;
  scatter: ScatterDef[];
}

export interface TileDef {
  id: string;
  name: string;
  kind: string;
  sourceSizePx: [number, number];
  coverageCells: [number, number];
  files: string[];
  transitionPolicy?: string;
}

export interface ObjectLayer {
  kind: string;
  file: string;
  sizePx: [number, number];
  pivotPx: [number, number];
  rootOffsetWorld?: [number, number, number];
  axis?: string;
  state?: string;
}

export interface ObjectDef {
  id: string;
  name: string;
  kind: string;
  placement: string;
  footprintCells: [number, number];
  originCells?: [number, number] | null;
  pixelsPerWorldUnit?: number;
  renderLayers: ObjectLayer[];
  structure?: { profile: 'log-wall' | 'plank-door'; height: number; thickness: number; texture: string } | null;
  blocksMovement?: boolean;
  blocksVision?: boolean;
  occludesVisually?: boolean;
  collisionPolygon?: [number, number][];
}

export interface CastFrame {
  file: string;
  sizePx: [number, number];
  pivotPx: [number, number];
}

export interface CastDef {
  id: string;
  name: string;
  pixelsPerWorldUnit?: number;
  suggestedFootprintCells?: [number, number];
  views: { north: CastFrame; east: CastFrame; south: CastFrame; west: CastFrame };
  portrait?: string | null;
  animations?: CastAnimations;
}

export interface CastAnimationClip { row: number; frames: number; fps: number }
export interface CastAnimationView { file: string; pixelsPerWorldUnit?: number; pivotPx?: [number, number] }
export interface CastAnimations {
  frameSizePx: [number, number];
  pivotPx: [number, number];
  pixelsPerWorldUnit: number;
  columns: number;
  rows: number;
  views: { north: CastAnimationView; east: CastAnimationView; south: CastAnimationView; west: CastAnimationView };
  clips: Record<'idle' | 'melee' | 'ranged' | 'move' | 'hurt' | 'spell', CastAnimationClip>;
}

/** A wide weapon can need a smaller raster scale than another direction. Its
 * matching PPU and foot pivot keep the character the same size in the world.
 */
export function resolveAnimationView(animations: CastAnimations | undefined, facing: string) {
  if (!animations) return null;
  const view = animations.views[facing as keyof CastAnimations['views']] ?? animations.views.south;
  return { file: view.file, definition: {
    ...animations,
    pixelsPerWorldUnit: view.pixelsPerWorldUnit ?? animations.pixelsPerWorldUnit,
    pivotPx: view.pivotPx ?? animations.pivotPx,
  } };
}

export interface Pack {
  id: string;
  name: string;
  pixelsPerCell: number;
  filtering: 'linear' | 'nearest';
  terrains: TerrainDef[];
  decals: DecalDef[];
  walls: WallDef[];
  doors: DoorDef[];
  props: PropDef[];
  themes: ThemeDef[];
  tiles?: TileDef[];
  objects?: ObjectDef[];
  cast?: CastDef[];
  schemaVersion?: number;
  visualProfile?: string;
  contentHash?: string;
  tray?: { felt?: string | null; wood?: string | null };
  baseUrl: string;
}

export interface AssetRef<T> {
  pack: Pack;
  local: string;
  def: T;
}

const textures = new Map<string, Promise<THREE.Texture | null>>();
const images = new Map<string, Promise<CanvasImageSource | null>>();
let imageDecodes = 0;
let loading: { revision: string; promise: Promise<Pack[]> } | null = null;
let loadedRevision = '';
let cachedPacks: Pack[] | null = null;

export function imageDecodeCount() {
  return imageDecodes;
}

/** One decode per URL. Painting does not decode the source again. */
export function cachedImage(url: string): Promise<CanvasImageSource | null> {
  const cached = images.get(url);
  if (cached) return cached;
  imageDecodes += 1;
  const pending = fetch(url)
    .then((response) => (response.ok ? response.blob() : null))
    .then((blob) => (blob ? createImageBitmap(blob) : null))
    .catch(() => null);
  images.set(url, pending);
  return pending;
}

/**
 * Load the library for a revision. A failed request is not stored, so the
 * next call tries again. An older response cannot replace a newer revision.
 */
export function ensureCatalog(revision: string): Promise<Pack[]> {
  if (cachedPacks && revision === loadedRevision) return Promise.resolve(cachedPacks);
  if (loading?.revision === revision) return loading.promise;
  const promise = fetch('/api/packs')
    .then((response) => {
      if (!response.ok) throw new Error('the asset library did not load');
      return response.json() as Promise<{ packs?: Pack[] }>;
    })
    .then((body) => {
      const packs = body.packs ?? [];
      if (loading?.promise === promise || !loading) {
        loadedRevision = revision;
        cachedPacks = packs;
        loading = null;
      }
      return packs;
    })
    .catch((error) => {
      if (loading?.promise === promise) loading = null;
      throw error;
    });
  loading = { revision, promise };
  return promise;
}

export function loadCatalog(): Promise<Pack[]> {
  if (cachedPacks) return Promise.resolve(cachedPacks);
  return ensureCatalog(loadedRevision);
}

export function noteCatalogRevision(revision: string): boolean {
  if (revision === loadedRevision && cachedPacks) return false;
  return true;
}

export function splitAsset(asset: string): { pack: string; local: string } | null {
  const slash = asset.indexOf('/');
  if (slash <= 0 || slash === asset.length - 1) return null;
  return { pack: asset.slice(0, slash), local: asset.slice(slash + 1) };
}

export function findIn<T extends { id: string }>(packs: Pack[], asset: string, list: (pack: Pack) => T[]): AssetRef<T> | null {
  const parts = splitAsset(asset);
  if (!parts) return null;
  const pack = packs.find((item) => item.id === parts.pack);
  const def = pack ? list(pack).find((item) => item.id === parts.local) : undefined;
  if (!pack || !def) return null;
  return { pack, local: parts.local, def };
}

/** Nine cells. Even places keep the inspected tile. Odd places use the neighbor, so the join is visible. */
export function seamCells(own: string, neighbor: string): string[] {
  return Array.from({ length: 9 }, (_, index) => (index % 2 === 0 ? own : neighbor));
}

export function fileUrl(pack: Pack, file: string | null | undefined): string | null {
  if (!file) return null;
  if (/^(blob:|data:|https?:)/.test(file)) return file;
  return `${pack.baseUrl}/${file}`;
}

/** Reimporting a sheet under the same filename must replace its cached artwork. */
export function animationFileUrl(pack: Pack, file: string | null | undefined): string | null {
  const url = fileUrl(pack, file);
  return url?.startsWith('/') && pack.contentHash ? `${url}${url.includes('?') ? '&' : '?'}v=${encodeURIComponent(pack.contentHash)}` : url;
}

export function loadTexture(url: string, filtering: Pack['filtering']): Promise<THREE.Texture | null> {
  const key = `${filtering}:${url}`;
  const cached = textures.get(key);
  if (cached) return cached;
  const pending = new Promise<THREE.Texture | null>((resolve) => {
    const loader = new THREE.TextureLoader();
    loader.load(
      url,
      (texture) => {
        texture.colorSpace = THREE.SRGBColorSpace;
        const filter = filtering === 'nearest' ? THREE.NearestFilter : THREE.LinearFilter;
        texture.magFilter = filter;
        texture.minFilter = filter;
        texture.wrapS = THREE.ClampToEdgeWrapping;
        texture.wrapT = THREE.ClampToEdgeWrapping;
        texture.needsUpdate = true;
        resolve(texture);
      },
      undefined,
      () => resolve(null),
    );
  });
  textures.set(key, pending);
  return pending;
}

export function trayTextures(packs: Pack[]): { felt: string | null; wood: string | null } {
  for (const pack of packs) {
    const felt = fileUrl(pack, pack.tray?.felt);
    const wood = fileUrl(pack, pack.tray?.wood);
    if (felt || wood) return { felt, wood };
  }
  return { felt: null, wood: null };
}
