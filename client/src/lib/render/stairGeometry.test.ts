import { expect, it } from 'vitest';
import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout';
import { cellLevel } from './levels';
import { stairGeometry } from './stairGeometry';
import { stairSurfaceHeight } from './stairShape';

const layout = (): MapLayout => ({
  width: 4, height: 3, terrains: ['stone'], cells: encodeCells(new Uint8Array(12)), props: [], pieces: [], decals: [], lights: [],
  levels: encodeCells(Uint8Array.from([0,0,1,1, 0,0,1,1, 0,0,1,1])),
  stairs: [{ id: 'stairs', asset: 'stone/steps', x: 1, y: 1, x2: 2, y2: 1 }],
});

it('puts seven real treads in front of the cliff and a landing on the higher floor', () => {
  const map = layout(), geometry = stairGeometry(map)!;
  const points = geometry.getAttribute('position');
  const tops = new Set<number>();
  for (let i = 0; i < points.count; i += 1) {
    const x = points.getX(i), y = points.getY(i);
    if (y < 1.019) expect(x).toBeLessThan(2);
    if (x < 2 && y > 0.03) tops.add(Math.round(y * 10_000));
  }
  expect(tops.size).toBe(7);
  geometry.computeBoundingBox();
  expect(geometry.boundingBox!.max.x).toBeGreaterThan(2.5);
  expect(geometry.boundingBox!.max.y).toBeCloseTo(1.02);
  expect(points.count).toBeLessThan(250);
  expect(geometry.groups).toHaveLength(1);
  geometry.dispose();
});

it('supports feet on the same treads in both stair directions without raising nearby ground', () => {
  const map = layout(), level = (x: number, y: number) => cellLevel(map, x, y);
  const ascending = stairSurfaceHeight(map, 1.5, 1.5, level);
  expect(ascending).toBeGreaterThan(0.4);
  expect(ascending).toBeLessThan(0.7);
  expect(stairSurfaceHeight(map, 2.5, 1.5, level)).toBe(1);
  expect(stairSurfaceHeight(map, 1.5, 2.2, level)).toBeNull();
  map.stairs = [{ id: 'stairs', asset: 'stone/steps', x: 2, y: 1, x2: 1, y2: 1 }];
  expect(stairSurfaceHeight(map, 1.5, 1.5, level)).toBe(ascending);
});
