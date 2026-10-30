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

export function opaqueTexel(image: CanvasImageSource | undefined, u: number, v: number): boolean {
  if (!image || typeof document === 'undefined') return true;
  const width = 'width' in image ? Number(image.width) : 0;
  const height = 'height' in image ? Number(image.height) : 0;
  if (!width || !height) return true;
  const canvas = document.createElement('canvas');
  canvas.width = 1;
  canvas.height = 1;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) return true;
  const x = Math.min(width - 1, Math.max(0, Math.floor(u * width)));
  const y = Math.min(height - 1, Math.max(0, Math.floor((1 - v) * height)));
  try {
    context.drawImage(image, x, y, 1, 1, 0, 0, 1, 1);
    return (context.getImageData(0, 0, 1, 1).data[3] ?? 0) >= ALPHA_CUTOFF * 255;
  } catch {
    return true;
  }
}
