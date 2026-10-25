// ------------------------------------------------------------
// Module: view
// Purpose: The one camera angle every upright picture is drawn
//          for. Ground north-south shows at 0.8, and a cell of
//          height shows at 0.6. docs/assets.md uses these values.
// ------------------------------------------------------------

export const ELEVATION_SIN = 0.8;
export const ELEVATION_COS = 0.6;
export const GROUND_DEPTH = 0.8;
export const HEIGHT_SCALE = 0.6;

/** Screen-up, as a world direction. Screen-right is world +X. */
export const SCREEN_UP = { x: 0, y: HEIGHT_SCALE, z: -GROUND_DEPTH };

export function uprightPixels(width: number, depth: number, height: number, pixelsPerCell: number) {
  return {
    width: Math.round(width * pixelsPerCell),
    height: Math.round(depth * GROUND_DEPTH * pixelsPerCell + height * HEIGHT_SCALE * pixelsPerCell),
  };
}

/** A camera-facing card shows the file at one pixel per P at the native zoom. */
export function standeeWorldSize(imageWidth: number, imageHeight: number, pixelsPerCell: number) {
  const scale = Math.max(1, pixelsPerCell);
  return { width: imageWidth / scale, height: imageHeight / scale };
}
