import { bitAt } from '../fog/bits';
import type { FogView } from '../net/bindings/FogView';
import { fogMasks } from './seen';

export const REMEMBERED_FOG_ALPHA = 184;

/** Paint into the retained canvas: LOS updates only upload pixels, not geometry. */
export function drawFog(fog: FogView, mapWidth: number, mapHeight: number, canvas = document.createElement('canvas')) {
  const width = Math.max(1, fog.width);
  const height = Math.max(1, fog.height);
  if (canvas.width !== width) canvas.width = width;
  if (canvas.height !== height) canvas.height = height;
  const context = canvas.getContext('2d');
  if (!context) return canvas;
  context.globalCompositeOperation = 'source-over';
  const image = context.createImageData(width, height);
  const { exploredBits, visibleBits } = fogMasks(fog);
  const gm = fog.mode === 'gm';
  for (let index = 0; index < width * height; index += 1) {
    const alpha = !gm && bitAt(visibleBits, index) ? 0
      : bitAt(exploredBits, index) ? (gm ? 24 : REMEMBERED_FOG_ALPHA) : (gm ? 58 : 255);
    const offset = index * 4;
    image.data[offset] = 6;
    image.data[offset + 1] = 5;
    image.data[offset + 2] = 4;
    image.data[offset + 3] = alpha;
  }
  context.putImageData(image, 0, 0);
  if (!gm && !fog.visible && fog.visiblePolygons.length > 0) {
    context.globalCompositeOperation = 'destination-out';
    context.fillStyle = '#000';
    for (const polygon of fog.visiblePolygons) {
      if (polygon.length < 3) continue;
      context.beginPath();
      polygon.forEach(([x, y], index) => {
        const px = (x / mapWidth) * width;
        const py = (y / mapHeight) * height;
        if (index === 0) context.moveTo(px, py);
        else context.lineTo(px, py);
      });
      context.closePath();
      context.fill();
    }
  }
  context.globalCompositeOperation = 'source-over';
  return canvas;
}
