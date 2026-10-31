import { describe, expect, it } from 'vitest';

import type { FogView } from '../net/bindings/FogView.ts';
import { fogStamp, sameFogPicture, sameFogStamp } from './fogStamp';

const fog = (extra: Partial<FogView> = {}): FogView => ({
  cellsPerSquare: 1,
  width: 4,
  height: 4,
  explored: 'AA==',
  visiblePolygons: [[[0, 0], [2, 0], [2, 2]]],
  mode: 'player',
  ...extra,
});

describe('sameFogStamp', () => {
  it('treats a copied lit shape as the same sheet', () => {
    const first = fog();
    const stamp = fogStamp(first, 12, '');
    const copy = fog({ visiblePolygons: [[[0, 0], [2, 0], [2, 2]]] });
    expect(sameFogStamp(stamp, copy, 12, '')).toBe(true);
  });

  it('notices when one corner of the lit shape moves', () => {
    const stamp = fogStamp(fog(), 12, '');
    const moved = fog({ visiblePolygons: [[[0, 0], [3, 0], [2, 2]]] });
    expect(sameFogStamp(stamp, moved, 12, '')).toBe(false);
  });

  it('notices a new remembered cell and a raised square', () => {
    const stamp = fogStamp(fog(), 12, '');
    expect(sameFogStamp(stamp, fog({ explored: 'AQ==' }), 12, '')).toBe(false);
    expect(sameFogStamp(stamp, fog(), 12, 'AQI=')).toBe(false);
  });

  it('treats a newly remembered cell as a different picture', () => {
    const before = fog();
    expect(sameFogPicture(before, fog())).toBe(true);
    expect(sameFogPicture(before, fog({ explored: 'AQ==' }))).toBe(false);
  });

  it('refreshes when the map height or subcell scale changes', () => {
    const before = fog();
    const stamp = fogStamp(before, 12, '', 8);
    expect(sameFogStamp(stamp, before, 12, '', 9)).toBe(false);
    expect(sameFogStamp(stamp, fog({ cellsPerSquare: 4 }), 12, '', 8)).toBe(false);
    expect(sameFogPicture(before, fog({ cellsPerSquare: 4 }))).toBe(false);
  });
});
