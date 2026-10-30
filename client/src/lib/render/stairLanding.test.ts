import { describe, expect, it } from 'vitest';

import { encodeCells } from '../editor/cells';
import { stairLanding } from './stairLanding';

const levels = new Uint8Array(16);
levels[1 * 4 + 2] = 1;
const layout = {
  width: 4,
  height: 4,
  levels: encodeCells(levels),
  stairs: [{ id: 's', asset: 'ruin/stair', x: 1, y: 1, x2: 2, y2: 1 }],
};

describe('stairLanding', () => {
  it('lands on the nearer end of the stair', () => {
    expect(stairLanding(layout, 1.6, 1.5)).toEqual({ x: 1.5, y: 1.5 });
    expect(stairLanding(layout, 2.3, 1.5)).toEqual({ x: 2.5, y: 1.5 });
  });

  it('lands on the higher square when the click is exactly in the middle', () => {
    expect(stairLanding(layout, 2, 1.5)).toEqual({ x: 2.5, y: 1.5 });
  });

  it('leaves a click beside the stair on the ground', () => {
    expect(stairLanding(layout, 1.5, 2.4)).toBeNull();
  });
});
