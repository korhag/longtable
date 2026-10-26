// ------------------------------------------------------------
// Module: projection
// Purpose: The same camera basis the exporter uses, so a picture
//          and the live table look along one direction.
// ------------------------------------------------------------

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export const CLASSIC_ELEVATION_DEG = 53.13010235415598;

export function basis(azimuthDeg: number, elevationDeg: number) {
  const azimuth = (azimuthDeg * Math.PI) / 180;
  const elevation = (elevationDeg * Math.PI) / 180;
  const sinE = Math.sin(elevation);
  const cosE = Math.cos(elevation);
  const sinA = Math.sin(azimuth);
  const cosA = Math.cos(azimuth);
  const forward = normalize({ x: sinA * cosE, y: -sinE, z: -cosA * cosE });
  const rawUp = { x: sinA * cosE * sinE, y: cosE * cosE, z: -cosA * cosE * sinE };
  const length = rawUp.x * rawUp.x + rawUp.y * rawUp.y + rawUp.z * rawUp.z;
  let up = length < 1e-16 ? { x: 0, y: 0, z: -1 } : normalize(rawUp);
  const right = normalize(cross(forward, up));
  up = normalize(cross(right, forward));
  return { forward, up, right };
}

/** Dragging the picture moves the focus in the ground plane. */
export function panGround(right: Vec3, up: Vec3, movementX: number, movementY: number, scale: number) {
  const span = up.x * up.x + up.z * up.z || 1;
  const alongRight = -movementX * scale;
  const alongScreen = movementY * scale;
  return {
    x: alongRight * right.x + (alongScreen * up.x) / span,
    z: alongRight * right.z + (alongScreen * up.z) / span,
  };
}

function cross(a: Vec3, b: Vec3): Vec3 {
  return {
    x: a.y * b.z - a.z * b.y,
    y: a.z * b.x - a.x * b.z,
    z: a.x * b.y - a.y * b.x,
  };
}

function normalize(value: Vec3): Vec3 {
  const length = Math.hypot(value.x, value.y, value.z) || 1;
  return { x: value.x / length, y: value.y / length, z: value.z / length };
}
