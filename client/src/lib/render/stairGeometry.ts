import * as THREE from 'three';
import type { MapLayout } from '../net/bindings/MapLayout';
import { cellLevel, SUPPORT_CLEARANCE } from './levels';
import { stairFrame, STAIR_LANDING, STAIR_RUN, STAIR_STEPS, STAIR_WIDTH } from './stairShape';

/** Closed stone treads and risers with a landing that meets the upper floor. */
export function stairGeometry(layout: MapLayout): THREE.BufferGeometry | null {
  const positions: number[] = [], colors: number[] = [], uvs: number[] = [];
  const groups: { start: number; count: number; materialIndex: number }[] = [];
  let materialIndex = 0;
  for (const stair of layout.stairs ?? []) {
    const frame = stairFrame(stair, (x, z) => cellLevel(layout, x, z));
    if (!frame) continue;
    const start = positions.length / 3;
    const point = (along: number, across: number, height: number) => [frame.x + frame.dx * along - frame.dz * across, height + SUPPORT_CLEARANCE, frame.z + frame.dz * along + frame.dx * across];
    const face = (points: number[][], shade: number, uvWidth = 1, uvDepth = 1) => {
      const uv = [[0, 0], [uvWidth, 0], [uvWidth, uvDepth], [0, uvDepth]];
      for (const index of [0, 1, 2, 0, 2, 3]) {
        positions.push(...points[index]!);
        colors.push(shade, shade * 0.98, shade * 0.93);
        uvs.push(...uv[index]!);
      }
    };
    const half = STAIR_WIDTH / 2;
    const top = (from: number, to: number, y: number, shade: number) => face([point(from, -half, y), point(from, half, y), point(to, half, y), point(to, -half, y)], shade, 1, Math.max(0.12, to - from));
    top(-0.12, 0, frame.low, 0.83);
    for (let step = 0; step < STAIR_STEPS; step += 1) {
      const from = STAIR_RUN * step / STAIR_STEPS;
      const to = STAIR_RUN * (step + 1) / STAIR_STEPS;
      const height = frame.low + (step + 1) / STAIR_STEPS;
      const previous = frame.low + step / STAIR_STEPS;
      top(from, to, height, step % 2 ? 0.94 : 1);
      face([point(from, -half, previous), point(from, half, previous), point(from, half, height), point(from, -half, height)], 0.62, 1, height - previous);
      face([point(from, -half, frame.low), point(to, -half, frame.low), point(to, -half, height), point(from, -half, height)], 0.73, to - from, height - frame.low);
      face([point(to, half, frame.low), point(from, half, frame.low), point(from, half, height), point(to, half, height)], 0.57, to - from, height - frame.low);
    }
    top(STAIR_RUN, STAIR_RUN + STAIR_LANDING, frame.high, 0.96);
    groups.push({ start, count: positions.length / 3 - start, materialIndex: materialIndex++ });
  }
  if (positions.length === 0) return null;
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
  geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
  for (const group of groups) geometry.addGroup(group.start, group.count, group.materialIndex);
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();
  return geometry;
}
