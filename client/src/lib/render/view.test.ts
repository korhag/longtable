// ------------------------------------------------------------
// Module: view.test
// Purpose: The standee card is the image size divided by P.
// ------------------------------------------------------------

import { describe, expect, it } from 'vitest';

import { standeeWorldSize, uprightPixels } from './view';

describe('view', () => {
  it('maps a picture to world units at one pixel per cell', () => {
    expect(uprightPixels(2, 1, 0.75, 128)).toEqual({ width: 256, height: 160 });
    expect(standeeWorldSize(256, 160, 128)).toEqual({ width: 2, height: 1.25 });
  });
});
