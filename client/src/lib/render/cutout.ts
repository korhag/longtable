// ------------------------------------------------------------
// Module: cutout
// Purpose: World-upright pictures for objects and cast. The
//          pivot pixel's center sits on the root. Transparent
//          pixels do not write depth.
// ------------------------------------------------------------

import * as THREE from 'three';

import { COS_ELEVATION, sceneBasis } from './cameraProfile';

/** Shared by the color pass, the outline mask, and picking. */
export const ALPHA_CUTOFF = 0.5;

const texelCache = new WeakMap<CanvasImageSource, Uint8ClampedArray>();
const texelSize = new WeakMap<CanvasImageSource, { width: number; height: number }>();

/** True when the texel at this UV is opaque enough to select or occlude. A missing image does not count as a miss. */
export function opaqueTexel(image: CanvasImageSource | null | undefined, u: number, v: number): boolean {
  if (!image) return true;
  const sample = texelSample(image);
  if (!sample) return true;
  const x = Math.min(sample.width - 1, Math.max(0, Math.floor(u * sample.width)));
  const y = Math.min(sample.height - 1, Math.max(0, Math.floor((1 - v) * sample.height)));
  return (sample.data[(y * sample.width + x) * 4 + 3] ?? 0) >= ALPHA_CUTOFF * 255;
}

function texelSample(image: CanvasImageSource): { data: Uint8ClampedArray; width: number; height: number } | null {
  const cached = texelCache.get(image);
  const size = texelSize.get(image);
  if (cached && size) return { data: cached, width: size.width, height: size.height };
  const width = 'width' in image && typeof image.width === 'number' ? image.width : 0;
  const height = 'height' in image && typeof image.height === 'number' ? image.height : 0;
  if (!width || !height) return null;
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) return null;
  context.drawImage(image, 0, 0);
  const data = context.getImageData(0, 0, width, height).data;
  texelCache.set(image, data);
  texelSize.set(image, { width, height });
  return { data, width, height };
}

const { right } = sceneBasis();
export const CUTOUT_RIGHT = new THREE.Vector3(right.x, right.y, right.z);
export const CUTOUT_UP = new THREE.Vector3(0, 1, 0);
export const CUTOUT_TOWARD = new THREE.Vector3().crossVectors(CUTOUT_RIGHT, CUTOUT_UP).normalize();

export function orientUpright(mesh: THREE.Object3D) {
  mesh.quaternion.setFromRotationMatrix(new THREE.Matrix4().makeBasis(CUTOUT_RIGHT, CUTOUT_UP, CUTOUT_TOWARD));
}

/** Pixel coordinates grow right and down. The pivot maps to local (0, 0). */
export function uprightGeometry(sizePx: [number, number], pivotPx: [number, number], pixelsPerWorldUnit: number) {
  const scale = Math.max(1, pixelsPerWorldUnit);
  const vertical = scale * COS_ELEVATION;
  const corners: [number, number][] = [
    [0, 0],
    [sizePx[0], 0],
    [sizePx[0], sizePx[1]],
    [0, sizePx[1]],
  ];
  const positions: number[] = [];
  const uvs: number[] = [];
  for (const [px, py] of corners) {
    positions.push((px - pivotPx[0]) / scale, -(py - pivotPx[1]) / vertical, 0);
    uvs.push(sizePx[0] === 0 ? 0 : px / sizePx[0], sizePx[1] === 0 ? 0 : 1 - py / sizePx[1]);
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
  geometry.setIndex([0, 1, 2, 0, 2, 3]);
  return geometry;
}

export function cutoutMaterial(texture: THREE.Texture | null, color: number, ground: boolean) {
  return new THREE.MeshBasicMaterial({
    map: texture,
    color,
    alphaTest: ALPHA_CUTOFF,
    transparent: ground,
    depthWrite: !ground,
    depthTest: true,
    side: THREE.DoubleSide,
  });
}
