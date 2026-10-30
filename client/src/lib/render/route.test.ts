// ------------------------------------------------------------
// Module: route.test
// Purpose: A blocked straight step walks around the wall. A
//          cliff with no stair is named, and the line sits on
//          each square.
// ------------------------------------------------------------

import { describe, expect, it } from 'vitest';

import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { walkRoute } from './route';

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
