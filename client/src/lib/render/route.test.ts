// ------------------------------------------------------------
// Module: route.test
// Purpose: A blocked straight step walks around the wall. A
//          cliff with no stair is named, and the line sits on
//          each square.
// ------------------------------------------------------------

import { describe, expect, it } from 'vitest';

import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { blockingSegments, walkRoute } from './route';

function layout(width: number, height: number, levels: string | null): MapLayout {
  return {
    width,
    height,
    terrains: ['ruin/grass'],
    cells: encodeCells(new Uint8Array(width * height)),
    pieces: [],
    props: [],
    decals: [],
    lights: [],
    seed: 1,
    levels: levels ?? undefined,
  };
}

describe('walk route', () => {
  it('goes around a wall and names a cliff', () => {
    const around = walkRoute(layout(4, 4, null), { x: 0.5, y: 0.5 }, { x: 1.5, y: 0.5 }, [
      { a: [1, 0], b: [1, 1], kind: 'wall' },
    ]);
    expect(around.note).toBeNull();
    expect(around.points.map((point) => [point.x, point.z])).toEqual([
      [0.5, 0.5],
      [0.5, 1.5],
      [1.5, 1.5],
      [1.5, 0.5],
    ]);

    const levels = new Uint8Array(4);
    levels[2] = 1;
    const cliff = walkRoute(layout(2, 2, encodeCells(levels)), { x: 0.5, y: 0.5 }, { x: 0.5, y: 1.5 }, []);
    expect(cliff.note).toBe('There are no stairs on that edge');
    expect(cliff.points[0]?.y).toBeCloseTo(0.04, 5);
    expect(cliff.points[cliff.points.length - 1]?.y).toBeCloseTo(1.04, 5);
  });

  it('refuses a wide body that would cross onto a rise', () => {
    const levels = new Uint8Array(64);
    for (let y = 0; y < 8; y += 1) {
      for (let x = 4; x < 8; x += 1) levels[y * 8 + x] = 1;
    }
    const walk = walkRoute(layout(8, 8, encodeCells(levels)), { x: 3, y: 5 }, { x: 5, y: 5 }, [], [2, 2]);
    expect(walk.note).toBe('That body does not fit on one level');
  });
});

const rectangle = (x: number, y: number, w: number, h: number): import('./route').Segment[] => [
  { a: [x, y], b: [x + w, y], kind: 'block' },
  { a: [x + w, y], b: [x + w, y + h], kind: 'block' },
  { a: [x + w, y + h], b: [x, y + h], kind: 'block' },
  { a: [x, y + h], b: [x, y], kind: 'block' },
];
it('routes the whole body around a small trunk at a grid corner and refuses occupied destinations', () => {
  const map = layout(8, 8, null),
    trunk = rectangle(2.8, 2.8, 0.4, 0.4);
  const route = walkRoute(map, { x: 0.5, y: 2.5 }, { x: 5.5, y: 2.5 }, trunk);
  expect(route.note).toBeNull();
  expect(route.points.length).toBeGreaterThan(2);
  expect(route.points.every((p) => !(p.x >= 2 && p.x < 4 && p.z >= 2 && p.z < 4))).toBe(true);
  expect(walkRoute(map, { x: 0.5, y: 2.5 }, { x: 3.5, y: 3.5 }, trunk).note).toBe('Something is in the way');
  expect(walkRoute(map, { x: 0.5, y: 2.5 }, { x: 3.5, y: 3.5 }, rectangle(2, 2, 4, 4)).note).toBe(
    'Something is in the way',
  );
});
it('does not squeeze a wide figure through a single-square gap', () => {
  const obstacles = [...rectangle(4, 0, 1, 4), ...rectangle(4, 5, 1, 5)];
  expect(walkRoute(layout(10, 10, null), { x: 2, y: 5 }, { x: 8, y: 5 }, obstacles, [2, 2]).note).not.toBeNull();
  const small = walkRoute(layout(10, 10, null), { x: 2.5, y: 4.5 }, { x: 8.5, y: 4.5 }, obstacles);
  expect(small.note).toBeNull();
});
it('finds stairs away from the direct line and does not cut a wall corner diagonally', () => {
  const levels = new Uint8Array(36);
  for (let y = 0; y < 6; y++) for (let x = 3; x < 6; x++) levels[y * 6 + x] = 1;
  const map = layout(6, 6, encodeCells(levels));
  map.stairs = [{ id: 'up', asset: 'stairs', x: 2, y: 0, x2: 3, y2: 0 }];
  const route = walkRoute(map, { x: 0.5, y: 2.5 }, { x: 4.5, y: 2.5 }, []);
  expect(route.note).toBeNull();
  expect(route.points.some((p) => p.x === 2.5 && p.z === 0.5)).toBe(true);
  expect(route.points.some((p) => p.x === 3.5 && p.z === 0.5)).toBe(true);
  const corner = walkRoute(layout(4, 4, null), { x: 0.5, y: 0.5 }, { x: 1.5, y: 1.5 }, [
    { a: [1, 0], b: [1, 1], kind: 'wall' },
  ]);
  expect(corner.points.map((p) => [p.x, p.z])).toEqual([
    [0.5, 0.5],
    [0.5, 1.5],
    [1.5, 1.5],
  ]);
});
it('uses a short diagonal route on the largest map and caches a repeated preview', () => {
  const map = layout(200, 200, null),
    segments: import('./route').Segment[] = [];
  const route = walkRoute(map, { x: 0.5, y: 0.5 }, { x: 199.5, y: 199.5 }, segments);
  expect(route.note).toBeNull();
  expect(route.points).toHaveLength(200);
  expect(walkRoute(map, { x: 0.5, y: 0.5 }, { x: 199.5, y: 199.5 }, segments)).toBe(route);
});

it('invalidates cached navigation when a cabin door opens', () => {
  const map = layout(8, 8, null);
  for (let x = 2; x < 5; x++) {
    map.pieces.push({ id: `north-${x}`, kind: 'wall', asset: 'wall', x, y: 2, axis: 'h', open: false });
    map.pieces.push({
      id: x === 3 ? 'door' : `south-${x}`,
      kind: x === 3 ? 'door' : 'wall',
      asset: 'wall',
      x,
      y: 5,
      axis: 'h',
      open: false,
    });
  }
  for (let y = 2; y < 5; y++) {
    map.pieces.push({ id: `west-${y}`, kind: 'wall', asset: 'wall', x: 2, y, axis: 'v', open: false });
    map.pieces.push({ id: `east-${y}`, kind: 'wall', asset: 'wall', x: 5, y, axis: 'v', open: false });
  }
  const doors: import('../net/bindings/DoorView').DoorView[] = [{ id: 'door', a: [3, 5], b: [4, 5], open: false }];
  const packs: import('../assets/catalog').Pack[] = [];
  const closed = blockingSegments(map, doors, packs);
  expect(blockingSegments(map, doors, packs)).toBe(closed);
  expect(walkRoute(map, { x: 3.5, y: 6.5 }, { x: 3.5, y: 4.5 }, closed).note).toBe('A wall is in the way');
  doors[0]!.open = true;
  const open = blockingSegments(map, doors, packs);
  expect(open).not.toBe(closed);
  expect(walkRoute(map, { x: 3.5, y: 6.5 }, { x: 3.5, y: 4.5 }, open).note).toBeNull();
});
