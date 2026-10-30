// ------------------------------------------------------------
// Module: grid.test
// Purpose: A raised square keeps the grid on its top. The line
//          does not stay at the floor and run through the block.
// ------------------------------------------------------------

import { describe, expect, it } from 'vitest';

import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { gridPositions } from './terrain';

function layout(): MapLayout {
  const levels = new Uint8Array(9);
  levels[1 * 3 + 1] = 2;
  return {
    width: 3,
    height: 3,
    terrains: ['ruin/grass'],
    cells: encodeCells(new Uint8Array(9)),
    pieces: [],
    props: [],
    decals: [],
    lights: [],
    seed: 1,
    levels: encodeCells(levels),
  };
}

function heightAt(points: number[], x: number, z: number) {
  for (let index = 0; index < points.length; index += 6) {
    const same = points[index] === x && points[index + 2] === z && points[index + 3] === x + 1 && points[index + 5] === z;
    if (same) return points[index + 1];
  }
  return null;
}

describe('raised grid', () => {
  it('lifts the line onto a rise and leaves the floor line on the floor', () => {
    const points = gridPositions(3, 3, layout());
    expect(heightAt(points, 1, 1)).toBeCloseTo(2.012, 5);
    expect(heightAt(points, 0, 0)).toBeCloseTo(0.012, 5);
  });
});
