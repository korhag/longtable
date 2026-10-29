// ------------------------------------------------------------
// Module: view.test
// Purpose: The standee card is the image size divided by P.
// ------------------------------------------------------------

import { describe, expect, it } from 'vitest';

import { standeeWorldSize, uprightPixels } from './view';

describe('view', () => {
  it('maps a picture to world units at one pixel per cell', () => {
    expect(uprightPixels(2, 1, 0.75, 128)).toEqual({ width: 256, height: 160 });
    const card = standeeWorldSize(256, 160, 128);
    expect(card.width).toBeCloseTo(2, 5);
    expect(card.height).toBeCloseTo(160 / (128 * Math.cos((30 * Math.PI) / 180)), 5);
  });
});
