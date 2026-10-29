// ------------------------------------------------------------
// Module: diagnostic
// Purpose: Asymmetric marks for the fixed camera. These are
//          owned test pictures, not production art.
// ------------------------------------------------------------

import { png } from './png';

export function groundPixels(x: number, y: number): [number, number, number, number] {
    if (x < 8 && y < 8) return [244, 241, 234, 255];
    if (x > 118 && y > 118) return [226, 59, 50, 255];
    if (x < 64) return [70, 92, 58, 255];
    return [92, 74, 48, 255];
}

export function groundTile() {
  return png(128, 128, groundPixels);
}

export function holePixels(x: number, y: number): [number, number, number, number] {
    const trunk = x > 26 && x < 38;
    const hole = x > 30 && x < 34 && y > 20 && y < 36;
    if (hole) return [0, 0, 0, 0];
    if (trunk || y < 28) return [42, 36, 28, 255];
    return [0, 0, 0, 0];
}

export function holedCutout() {
  return png(64, 96, holePixels);
}

export function castPixels(x: number, y: number): [number, number, number, number] {
    const body = x > 10 && x < 22 && y > 16 && y < 58;
    const head = (x - 16) ** 2 + (y - 12) ** 2 < 36;
    if (body || head) return [196, 168, 120, 255];
    return [0, 0, 0, 0];
}

export function castSilhouette() {
  return png(32, 64, castPixels);
}
