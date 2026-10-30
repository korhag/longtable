// ------------------------------------------------------------
// Module: cameraProfile
// Purpose: The one scene camera. Elevation 30°, azimuth 45° in
//          the existing basis() convention, which puts the camera
//          on the south-west side of the focus. Pan and zoom
//          change the focus and the scale, never this basis.
// ------------------------------------------------------------

import { basis, panGround, type Vec3 } from './projection';

export const PROFILE_ID = 'dimetric-30-v1';
export const AZIMUTH_DEG = 45;
export const ELEVATION_DEG = 30;
export const COS_ELEVATION = Math.cos((ELEVATION_DEG * Math.PI) / 180);

export interface WorldPoint {
  x: number;
  y: number;
  z: number;
}

export function sceneBasis() {
  return basis(AZIMUTH_DEG, ELEVATION_DEG);
}

/** A named profile is accepted only when it is this one, or still unmarked. */
export function acceptedProfile(value: string | null | undefined) {
  return value == null || value === '' || value === PROFILE_ID;
}

export function project(world: WorldPoint, focus: WorldPoint, origin: { u: number; v: number }, scale: number) {
  const { right, up } = sceneBasis();
  const dx = world.x - focus.x;
  const dy = world.y - focus.y;
  const dz = world.z - focus.z;
  return {
    u: origin.u + scale * dot({ x: dx, y: dy, z: dz }, right),
    v: origin.v - scale * dot({ x: dx, y: dy, z: dz }, up),
  };
}

/** Invert a screen point onto the horizontal plane at world height `planeY`. */
export function pickPlane(screen: { u: number; v: number }, focus: WorldPoint, origin: { u: number; v: number }, scale: number, planeY: number): WorldPoint {
  const { forward, up, right } = sceneBasis();
  const alongRight = (screen.u - origin.u) / scale;
  const alongUp = -(screen.v - origin.v) / scale;
  const rise = planeY - focus.y;
  const depth = (rise - alongRight * right.y - alongUp * up.y) / forward.y;
  return {
    x: focus.x + alongRight * right.x + alongUp * up.x + depth * forward.x,
    y: planeY,
    z: focus.z + alongRight * right.z + alongUp * up.z + depth * forward.z,
  };
}

/** Screen size of one square cell. Width:height is exactly 2:1. */
export function projectedCell(scale: number) {
  const width = scale * Math.SQRT2;
  const height = scale / Math.SQRT2;
  return { width, height, ratio: width / height };
}

/** Camera-space span of a map rectangle, including a raised top. */
export function projectedSpan(width: number, depth: number, rise: number) {
  const { right, up } = sceneBasis();
  let minRight = Infinity;
  let maxRight = -Infinity;
  let minUp = Infinity;
  let maxUp = -Infinity;
  for (const y of [0, rise]) {
    for (const x of [0, width]) {
      for (const z of [0, depth]) {
        const point = { x, y, z };
        const alongRight = dot(point, right);
        const alongUp = dot(point, up);
        minRight = Math.min(minRight, alongRight);
        maxRight = Math.max(maxRight, alongRight);
        minUp = Math.min(minUp, alongUp);
        maxUp = Math.max(maxUp, alongUp);
      }
    }
  }
  return { width: maxRight - minRight, height: maxUp - minUp };
}

/** Orthographic full height so the span fits the visible area inside the docks. */
export function framedDistance(
  spanWidth: number,
  spanHeight: number,
  canvasWidth: number,
  canvasHeight: number,
  insets: { left: number; right: number; top: number; bottom: number },
  pad = 1.08,
) {
  const visW = Math.max(1, canvasWidth - insets.left - insets.right);
  const visH = Math.max(1, canvasHeight - insets.top - insets.bottom);
  const aspect = visW / visH;
  const needH = spanHeight * pad;
  const needW = spanWidth * pad;
  return Math.max(needH, needW / aspect) * (canvasHeight / visH);
}

export function pan(movementX: number, movementY: number, scale: number) {
  const { right, up } = sceneBasis();
  return panGround(right, up, movementX, movementY, scale);
}

/**
 * Pull, near, and far for a map the focus can pan across.
 * The fixed basis does not change. Tall scenery is included in `rise`.
 */
export function cameraDepth(width: number, height: number, rise = 0) {
  const { forward } = sceneBasis();
  const points: WorldPoint[] = [];
  for (const y of [0, Math.max(0, rise)]) {
    for (const x of [0, width]) {
      for (const z of [0, height]) points.push({ x, y, z });
    }
  }
  let minRel = Infinity;
  let maxRel = -Infinity;
  for (const focus of points) {
    if (focus.y !== 0) continue;
    for (const point of points) {
      const rel = (point.x - focus.x) * forward.x + (point.y - focus.y) * forward.y + (point.z - focus.z) * forward.z;
      minRel = Math.min(minRel, rel);
      maxRel = Math.max(maxRel, rel);
    }
  }
  const near = 0.05;
  const margin = 2;
  const pull = Math.max(8, margin - minRel);
  const far = pull + maxRel + margin;
  return { pull, near, far };
}

/** A world-upright cutout offset from its pixel pivot. `v` grows down the image. */
export function cutoutOffset(uPx: number, vPx: number, pixelsPerWorldUnit: number) {
  const scale = Math.max(1, pixelsPerWorldUnit);
  return {
    horizontal: uPx / scale,
    vertical: -vPx / (scale * COS_ELEVATION),
  };
}

function dot(a: Vec3, b: Vec3) {
  return a.x * b.x + a.y * b.y + a.z * b.z;
}
