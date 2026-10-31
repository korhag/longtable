import { describe, expect, it } from 'vitest';

import type { FogView } from '../net/bindings/FogView.ts';
import { blockerBrightness, edgeBrightness, fogMasks, inSight, sceneryBrightness, seen } from './seen';

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

  it('never outlines an unowned actor in remembered darkness', () => {
    const remembered = fog({ explored: btoa(String.fromCharCode(255, 255)), visible: 'AA==' });
    expect(seen(remembered, 1.5, 1.5)).toBe(true);
    expect(inSight({ x: 1.5, y: 1.5, ownerId: 'bob' }, remembered, 'ada')).toBe(false);
    expect(sceneryBrightness(remembered, 1.5, 1.5)).toBeGreaterThan(0);
    expect(sceneryBrightness(remembered, 1.5, 1.5)).toBeLessThan(0.1);
  });

  it('samples actual roots at subcell resolution and rejects coordinates outside the bitmap', () => {
    const mask = fog({ cellsPerSquare: 4, explored: btoa(String.fromCharCode(0b0000_0100)), visible: 'AA==' });
    expect(seen(mask, 0.3, 0.3)).toBe(true);
    expect(seen(mask, 0.8, 0.8)).toBe(false);
    // Cell x=5 would alias the lit x=1,y=1 without an explicit bounds check.
    expect(seen(mask, 1.3, 0.05)).toBe(false);
    expect(seen(mask, -0.1, 0.3)).toBe(false);
  });

  it('reuses decoded masks and refreshes only the changed picture', () => {
    const view = fog({ explored: 'AA==', visible: 'AA==' });
    expect(fogMasks(view)).toBe(fogMasks(view));
    const before = fogMasks(view);
    view.visible = 'gA==';
    expect(fogMasks(view)).not.toBe(before);
    expect(sceneryBrightness(view, 0.2, 0.2)).toBe(1);
    expect(sceneryBrightness(view, 2.2, 2.2)).toBe(0);
  });

  it('keeps the near face of a wall visible while its far side is dark', () => {
    const view = fog({ explored: 'gA==', visible: 'gA==' });
    expect(sceneryBrightness(view, 1, 0.5)).toBe(0);
    expect(edgeBrightness(view, 1, 0.5, 'v')).toBe(1);
  });

  it('shows a blocking rock from its visible near face without revealing an actor behind it', () => {
    const view = fog({ explored: 'gA==', visible: 'gA==' });
    expect(sceneryBrightness(view, 1.3, 0.5)).toBe(0);
    expect(blockerBrightness(view, 1.3, 0.5, [1, 1])).toBe(1);
    expect(inSight({ x: 1.3, y: 0.5 }, view)).toBe(false);
  });
});
