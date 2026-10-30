import { describe, expect, it } from 'vitest';

import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { decodeCells, encodeCells } from './cells';
import { clearVariantPins, pinVariant } from './variants';

function layout(): MapLayout {
  return {
    width: 2,
    height: 1,
    terrains: ['demo/floor'],
    cells: encodeCells(new Uint8Array([0, 0])),
    pieces: [],
    props: [],
    decals: [],
    lights: [],
    seed: 7,
  };
}

describe('saved tile pictures', () => {
  it('pins one square and lets a new brush clear it', () => {
    const pinned = pinVariant(layout(), 0, 0, 'demo/floor', 3);
    expect(pinned.stored).toBeGreaterThan(0);
    const bytes = decodeCells(pinned.layout.variants ?? '');
    expect(bytes[0]).toBe(pinned.stored);
    expect(bytes[1]).toBe(0);
    const cleared = clearVariantPins(pinned.layout, [{ x: 0, y: 0 }]);
    expect(cleared.variants).toBeUndefined();
  });

  it('leaves a neighboring pin when another square is painted', () => {
    const first = pinVariant(layout(), 1, 0, 'demo/floor', 3);
    const cleared = clearVariantPins(first.layout, [{ x: 0, y: 0 }]);
    expect(decodeCells(cleared.variants ?? '')[1]).toBe(first.stored);
  });
});
