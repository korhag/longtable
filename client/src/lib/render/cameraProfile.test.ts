import { describe, expect, it } from 'vitest';

import {
  acceptedProfile,
  AZIMUTH_DEG,
  cameraDepth,
  centeredSpan,
  cutoutOffset,
  ELEVATION_DEG,
  framedDistance,
  pan,
  pickPlane,
  PROFILE_ID,
  project,
  projectedCell,
  projectedSpan,
  sceneBasis,
} from './cameraProfile';

describe('dimetric-30-v1', () => {
  const focus = { x: 4, y: 0, z: 6 };
  const origin = { u: 200, v: 120 };
  const scale = 128;

  it('is an orthonormal south-west basis at 30° and 45°', () => {
    expect(AZIMUTH_DEG).toBe(45);
    expect(ELEVATION_DEG).toBe(30);
    expect(PROFILE_ID).toBe('dimetric-30-v1');
    const { forward, up, right } = sceneBasis();
    expect(forward.x).toBeCloseTo(0.612372436, 6);
    expect(forward.y).toBeCloseTo(-0.5, 6);
    expect(forward.z).toBeCloseTo(-0.612372436, 6);
    expect(up.x).toBeCloseTo(0.353553391, 6);
    expect(up.y).toBeCloseTo(0.866025404, 6);
    expect(up.z).toBeCloseTo(-0.353553391, 6);
    expect(right.x).toBeCloseTo(0.707106781, 6);
    expect(right.y).toBeCloseTo(0, 6);
    expect(right.z).toBeCloseTo(0.707106781, 6);
    for (const axis of [forward, up, right]) {
      expect(Math.hypot(axis.x, axis.y, axis.z)).toBeCloseTo(1, 6);
    }
    expect(forward.x * up.x + forward.y * up.y + forward.z * up.z).toBeCloseTo(0, 6);
    expect(forward.x * right.x + forward.y * right.y + forward.z * right.z).toBeCloseTo(0, 6);
    const camera = { x: focus.x - forward.x, y: focus.y - forward.y, z: focus.z - forward.z };
    expect(camera.x).toBeLessThan(focus.x);
    expect(camera.z).toBeGreaterThan(focus.z);
  });

  it('projects a square cell as a 2:1 diamond with equal ground axes', () => {
    const cell = projectedCell(scale);
    expect(cell.ratio).toBeCloseTo(2, 8);
    expect(cell.width).toBeCloseTo(181.01933598, 4);
    expect(cell.height).toBeCloseTo(90.50966799, 4);
    const east = project({ x: focus.x + 1, y: 0, z: focus.z }, focus, origin, scale);
    const south = project({ x: focus.x, y: 0, z: focus.z + 1 }, focus, origin, scale);
    const eastLen = Math.hypot(east.u - origin.u, east.v - origin.v);
    const southLen = Math.hypot(south.u - origin.u, south.v - origin.v);
    expect(eastLen).toBeCloseTo(southLen, 6);
  });

  it('picks the same ground point it projected, on every tier', () => {
    for (const tier of [0, 1, 2]) {
      const world = { x: 3.25, y: tier, z: 7.5 };
      const screen = project(world, focus, origin, scale);
      const picked = pickPlane(screen, focus, origin, scale, tier);
      expect(picked.x).toBeCloseTo(world.x, 6);
      expect(picked.y).toBeCloseTo(tier, 6);
      expect(picked.z).toBeCloseTo(world.z, 6);
    }
  });

  it('pans both map axes and keeps a ground point under the cursor when zooming', () => {
    const shift = pan(12, -4, 0.25);
    expect(shift.x).not.toBeCloseTo(0, 5);
    expect(shift.z).not.toBeCloseTo(0, 5);
    const world = { x: 5.5, y: 0, z: 2.25 };
    const screen = project(world, focus, origin, scale);
    const closer = 160;
    const drifted = pickPlane(screen, focus, origin, closer, 0);
    const corrected = {
      x: focus.x + world.x - drifted.x,
      y: 0,
      z: focus.z + world.z - drifted.z,
    };
    const again = pickPlane(screen, corrected, origin, closer, 0);
    expect(again.x).toBeCloseTo(world.x, 5);
    expect(again.z).toBeCloseTo(world.z, 5);
  });

  it('frames a larger distance when the docks cover the canvas', () => {
    const span = projectedSpan(12, 12, 2);
    const open = framedDistance(span.width, span.height, 1200, 800, { left: 0, right: 0, top: 0, bottom: 0 });
    const docked = framedDistance(span.width, span.height, 1200, 800, { left: 280, right: 320, top: 48, bottom: 220 });
    expect(docked).toBeGreaterThan(open);
    expect(span.height).toBeGreaterThan(projectedSpan(12, 12, 0).height);
  });

  it('keeps a raised corner inside a frame aimed at the ground', () => {
    const flat = centeredSpan(12, 12, 0);
    const floor = projectedSpan(12, 12, 0);
    expect(flat.width).toBeCloseTo(floor.width, 5);
    expect(flat.height).toBeCloseTo(floor.height, 5);
    const raised = centeredSpan(12, 12, 3);
    expect(raised.height).toBeGreaterThan(flat.height);
    const { right, up } = sceneBasis();
    const focus = { x: 6, y: 0, z: 6 };
    for (const y of [0, 3]) {
      for (const x of [0, 12]) {
        for (const z of [0, 12]) {
          const rel = { x: x - focus.x, y, z: z - focus.z };
          const alongRight = rel.x * right.x + rel.y * right.y + rel.z * right.z;
          const alongUp = rel.x * up.x + rel.y * up.y + rel.z * up.z;
          expect(Math.abs(alongRight)).toBeLessThanOrEqual(raised.width / 2 + 0.001);
          expect(Math.abs(alongUp)).toBeLessThanOrEqual(raised.height / 2 + 0.001);
        }
      }
    }
  });

  it('keeps every corner of a 200 by 200 map between the near and far planes', () => {
    const width = 200;
    const height = 200;
    const rise = 4;
    const depth = cameraDepth(width, height, rise);
    const { forward } = sceneBasis();
    const points = [];
    for (const y of [0, rise]) {
      for (const x of [0, width]) {
        for (const z of [0, height]) points.push({ x, y, z });
      }
    }
    for (const focus of points) {
      if (focus.y !== 0) continue;
      const camera = {
        x: focus.x - forward.x * depth.pull,
        y: focus.y - forward.y * depth.pull,
        z: focus.z - forward.z * depth.pull,
      };
      for (const point of points) {
        const along =
          (point.x - camera.x) * forward.x +
          (point.y - camera.y) * forward.y +
          (point.z - camera.z) * forward.z;
        expect(along).toBeGreaterThan(depth.near);
        expect(along).toBeLessThan(depth.far);
      }
    }
  });

  it('refuses any other scene camera profile', () => {
    expect(acceptedProfile(PROFILE_ID)).toBe(true);
    expect(acceptedProfile('')).toBe(true);
    expect(acceptedProfile(null)).toBe(true);
    expect(acceptedProfile('classic')).toBe(false);
    expect(acceptedProfile('perspective')).toBe(false);
  });

  it('scales a cutout by the elevation cosine, not by a flat card', () => {
    const offset = cutoutOffset(64, -128, 128);
    expect(offset.horizontal).toBeCloseTo(0.5, 6);
    expect(offset.vertical).toBeCloseTo(128 / (128 * Math.cos((30 * Math.PI) / 180)), 6);
  });
});
