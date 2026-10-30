import { describe, expect, it } from 'vitest';

import { encodeCells } from '../editor/cells';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { fogShell } from './terrain';

describe('the largest map', () => {
  it('builds a fog shell for a 200-square map', () => {
    const flat = fogShell(200, 200, null);
    expect(flat.lifted).toBe(false);
    expect(flat.geometry.getAttribute('position').count).toBe(4);
    flat.geometry.dispose();

    const levels = new Uint8Array(200 * 200);
    levels[0] = 2;
    const layout: MapLayout = {
      width: 200,
      height: 200,
      terrains: ['core/grass'],
      cells: encodeCells(new Uint8Array(200 * 200)),
      pieces: [],
      props: [],
      decals: [],
      lights: [],
      levels: encodeCells(levels),
      seed: 1,
      stairs: [],
    };
    const raised = fogShell(200, 200, layout);
    expect(raised.lifted).toBe(true);
    expect(raised.geometry.getAttribute('position').count).toBeGreaterThan(4);
    raised.geometry.dispose();
  });
});
