import { describe, expect, it } from 'vitest';
import { panelBounds } from './panelBounds';

describe('floating panel bounds', () => {
  it('keeps a restored desktop sheet fully reachable on a small screen', () => {
    expect(panelBounds({ x: 1200, y: 800, w: 760, h: 680 }, 390, 650)).toEqual({ x: 8, y: 8, w: 374, h: 634 });
  });
  it('clamps dragging and preserves dimensions inside the viewport', () => {
    expect(panelBounds({ x: -80, y: 850, w: 380, h: 520 }, 1400, 900)).toEqual({ x: 8, y: 372, w: 380, h: 520 });
    expect(panelBounds({ x: 100, y: 72, w: 380, h: 520 }, 1400, 900)).toEqual({ x: 100, y: 72, w: 380, h: 520 });
  });
});
