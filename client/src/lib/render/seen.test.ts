import { describe, expect, it } from 'vitest';

import type { FogView } from '../net/bindings/FogView.ts';
import { inSight } from './seen';

const fog = (extra: Partial<FogView>): FogView => ({
  cellsPerSquare: 1,
  width: 4,
  height: 4,
  explored: '',
  visiblePolygons: [],
  mode: 'player',
  ...extra,
});

describe('inSight', () => {
  it('keeps a figure inside the light and drops one outside it', () => {
    const light = fog({ visiblePolygons: [[[0, 0], [2, 0], [2, 2], [0, 2]]] });
    expect(inSight({ x: 1, y: 1, ownerId: null }, light, 'ada')).toBe(true);
    expect(inSight({ x: 3.5, y: 3.5, ownerId: null }, light, 'ada')).toBe(false);
    expect(inSight({ x: 3.5, y: 3.5, ownerId: 'ada' }, light, 'ada')).toBe(true);
    expect(inSight({ x: 3.5, y: 3.5, ownerId: null }, fog({ mode: 'gm' }), 'ada')).toBe(true);
  });

  it('uses the sight mask when the table sends one', () => {
    const mask = fog({ visible: btoa(String.fromCharCode(0b1000_0000)) });
    expect(inSight({ x: 0.2, y: 0.2, ownerId: null }, mask, 'ada')).toBe(true);
    expect(inSight({ x: 1.2, y: 0.2, ownerId: null }, mask, 'ada')).toBe(false);
  });
});
