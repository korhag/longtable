import { describe, expect, it } from 'vitest';

import { encodeCells } from './cells';
import { fillTheme, footprintsOverlap } from './scatter';
import type { MapLayout } from '../net/bindings/MapLayout.ts';

function empty(width: number, height: number): MapLayout {
  return {
    width,
    height,
    terrains: ['core/grass'],
    cells: encodeCells(new Uint8Array(width * height)),
    pieces: [],
    props: [],
    decals: [],
    lights: [],
  };
}

describe('scatter', () => {
  it('repeats a seed, keeps spacing, and does not stack footprints', () => {
    const layout = empty(16, 16);
    const cells = [];
    for (let y = 0; y < 16; y += 1) {
      for (let x = 0; x < 16; x += 1) cells.push({ x, y });
    }
    const entries = [{ asset: 'core/oak', density: 4, spacing: 3, kind: 'prop' as const, footprint: [2, 2] as [number, number], rotate: false }];
    const once = fillTheme(layout, cells, 'core/grass', entries, 7);
    const twice = fillTheme(layout, cells, 'core/grass', entries, 7);
    expect(once.props.map((prop) => [prop.x, prop.y])).toEqual(twice.props.map((prop) => [prop.x, prop.y]));
    expect(once.props.length).toBeGreaterThan(1);
    for (let i = 0; i < once.props.length; i += 1) {
      for (let j = i + 1; j < once.props.length; j += 1) {
        const a = once.props[i]!;
        const b = once.props[j]!;
        expect(Math.hypot(a.x - b.x, a.y - b.y)).toBeGreaterThanOrEqual(3 - 1e-6);
        expect(footprintsOverlap({ x: a.x, y: a.y, w: 2, d: 2 }, { x: b.x, y: b.y, w: 2, d: 2 })).toBe(false);
      }
    }
  });
});

it('keeps large objects inside the painted region and off split elevations', () => {
  const layout = empty(8, 8);
  const levels = new Uint8Array(64);
  for (let y = 0; y < 8; y += 1) levels[y * 8 + 3] = 1;
  layout.levels = encodeCells(levels);
  const cells = Array.from({ length: 32 }, (_, i) => ({ x: i % 4, y: Math.floor(i / 4) }));
  const painted = fillTheme(layout, cells, 'forest/forest-floor', [{ asset: 'forest/oak', density: 8, spacing: 0, kind: 'prop', footprint: [2, 2], rotate: false }], 9);
  expect(painted.props.length).toBeGreaterThan(0);
  for (const prop of painted.props) {
    expect(prop.x).toBeLessThanOrEqual(1);
    expect(prop.y).toBeLessThanOrEqual(6);
  }
});

it('repainting a complete biome does not accumulate extra objects', () => {
  const layout = empty(20, 20);
  const cells = Array.from({ length: 400 }, (_, i) => ({ x: i % 20, y: Math.floor(i / 20) }));
  const entries = [{ asset: 'forest/birch', density: 0.2, spacing: 2, kind: 'prop' as const, footprint: [1, 1] as [number, number], rotate: false }];
  const once = fillTheme(layout, cells, 'forest/forest-floor', entries, 7);
  expect(once.props).toHaveLength(8);
  const twice = fillTheme(once, cells, 'forest/forest-floor', entries, 19);
  expect(twice.props).toEqual(once.props);
});
