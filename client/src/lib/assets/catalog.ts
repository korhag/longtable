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
  tray?: { felt?: string | null; wood?: string | null };
  baseUrl: string;
}

export interface AssetRef<T> {
  pack: Pack;
  local: string;
  def: T;
}

const textures = new Map<string, Promise<THREE.Texture | null>>();
let loading: Promise<Pack[]> | null = null;

export function loadCatalog(): Promise<Pack[]> {
  loading ??= fetch('/api/packs')
    .then((response) => (response.ok ? response.json() : { packs: [] }))
    .then((body: { packs?: Pack[] }) => body.packs ?? [])
    .catch(() => []);
  return loading;
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

export function fileUrl(pack: Pack, file: string | null | undefined): string | null {
  if (!file) return null;
  return `${pack.baseUrl}/${file}`;
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
