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
