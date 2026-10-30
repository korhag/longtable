import { describe, expect, it } from 'vitest';

import { encodeCells } from '../editor/cells';
import { cellLevel, FOG_LIFT, sceneRise, SUPPORT_CLEARANCE, supportHeight, tierLabel } from './levels';

describe('cellLevel', () => {
  it('reads a raised square and leaves the floor at zero', () => {
    const bytes = new Uint8Array(4);
    bytes[3] = 2;
    const layout = { width: 2, height: 2, levels: encodeCells(bytes) };
    expect(cellLevel(layout, 1.2, 1.8)).toBe(2);
    expect(cellLevel(layout, 0.2, 0.2)).toBe(0);
    expect(cellLevel({ width: 2, height: 2 }, 0, 0)).toBe(0);
    expect(supportHeight(layout, 1.2, 1.8)).toBeCloseTo(2.02);
    expect(sceneRise(layout)).toBe(3);
    expect(sceneRise(null)).toBe(1);
    expect(tierLabel(0)).toBe('On the floor');
    expect(tierLabel(1)).toBe('On a step');
    expect(tierLabel(2)).toBe('On a rise');
    expect(FOG_LIFT).toBeLessThan(SUPPORT_CLEARANCE);
  });
});
