import { describe, expect, it } from 'vitest';

import { castSilhouette, groundTile, holedCutout } from './diagnostic';
import { pngSize } from './png';

describe('diagnostic fixtures', () => {
  it('builds the same asymmetric pictures every time', () => {
    const ground = groundTile();
    const cutout = holedCutout();
    const cast = castSilhouette();
    expect(pngSize(ground)).toEqual({ width: 128, height: 128 });
    expect(pngSize(cutout)).toEqual({ width: 64, height: 96 });
    expect(pngSize(cast)).toEqual({ width: 32, height: 64 });
    expect(groundTile()).toEqual(ground);
    expect(holedCutout()).toEqual(cutout);
    expect(castSilhouette()).toEqual(cast);
    expect(ground[0]).toBe(137);
  });
});
