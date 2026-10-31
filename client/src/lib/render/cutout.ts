// ------------------------------------------------------------
// Module: cutout
// Purpose: One placement rule for upright pictures. The pivot
//          pixel sits on the object's root. Vertical pixels are
//          stretched by 1/cos(30°) so the picture keeps its
//          screen shape under the fixed camera.
// ------------------------------------------------------------

import * as THREE from 'three';

import { COS_ELEVATION, sceneBasis } from './cameraProfile';

export const ALPHA_CUTOFF = 0.5;

const basis = sceneBasis();
export const CUTOUT_RIGHT = new THREE.Vector3(basis.right.x, basis.right.y, basis.right.z);
export const CUTOUT_UP = new THREE.Vector3(0, 1, 0);
export const CUTOUT_TOWARD = new THREE.Vector3().crossVectors(CUTOUT_RIGHT, CUTOUT_UP).normalize();

export function cutoutPlacement(sizePx: [number, number], pivotPx: [number, number], pixelsPerWorldUnit: number) {
  const scale = Math.max(1, pixelsPerWorldUnit);
  const width = sizePx[0] / scale;
  const height = sizePx[1] / (scale * COS_ELEVATION);
  return {
    width,
    height,
    offsetX: width / 2 - pivotPx[0] / scale,
    offsetY: pivotPx[1] / (scale * COS_ELEVATION) - height / 2,
  };
}

export function uprightGeometry(sizePx: [number, number], pivotPx: [number, number], pixelsPerWorldUnit: number) {
  const place = cutoutPlacement(sizePx, pivotPx, pixelsPerWorldUnit);
  const geometry = new THREE.PlaneGeometry(place.width, place.height);
  geometry.translate(place.offsetX, place.offsetY, 0);
  return geometry;
}

export function orientUpright(object: THREE.Object3D) {
  object.quaternion.setFromRotationMatrix(new THREE.Matrix4().makeBasis(CUTOUT_RIGHT, CUTOUT_UP, CUTOUT_TOWARD));
}

export function cutoutMaterial(map: THREE.Texture | null, color: number, ground: boolean) {
  return new THREE.MeshBasicMaterial({
    map,
    color,
    transparent: true,
    alphaTest: ALPHA_CUTOFF,
    depthWrite: !ground,
    depthTest: true,
  });
}

// Weak keys let discarded artwork and its mask be reclaimed together.
const alphaMasks = new WeakMap<object, { width: number; height: number; alpha: Uint8Array; top: number } | null>();
export function opaqueTexel(image: CanvasImageSource | undefined, u: number, v: number): boolean {
  if (!image || typeof document === 'undefined') return true;
  const width = 'width' in image ? Number(image.width) : 0;
  const height = 'height' in image ? Number(image.height) : 0;
  if (!width || !height) return true;
  let mask = alphaMasks.get(image);
  if (mask === undefined) {
    const canvas = document.createElement('canvas');
    canvas.width = width; canvas.height = height;
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) return true;
    try {
      context.drawImage(image, 0, 0);
      const rgba = context.getImageData(0, 0, width, height).data;
      const alpha = new Uint8Array(width * height);
      let top = height;
      for (let i = 0; i < alpha.length; i += 1) {
        alpha[i] = rgba[i * 4 + 3] ?? 0;
        if (alpha[i]! >= ALPHA_CUTOFF * 255) top = Math.min(top, Math.floor(i / width));
      }
      mask = { width, height, alpha, top };
    } catch { mask = null; }
    alphaMasks.set(image, mask);
  }
  if (!mask) return true;
  const x = Math.min(mask.width - 1, Math.max(0, Math.floor(u * mask.width)));
  const y = Math.min(mask.height - 1, Math.max(0, Math.floor((1 - v) * mask.height)));
  return (mask.alpha[y * mask.width + x] ?? 0) >= ALPHA_CUTOFF * 255;
}

/** Share the picking mask with rigs, so transparent headroom is not body height. */
export function opaqueTop(image: CanvasImageSource | undefined): number | null {
  if (!image) return null;
  opaqueTexel(image, 0, 0);
  const mask = alphaMasks.get(image);
  return mask && mask.top < mask.height ? mask.top / mask.height : null;
}
