import { describe, expect, it } from 'vitest';

import { cutoutPlacement } from './cutout';

describe('cutout placement', () => {
  it('puts a centered foot pivot on the root and keeps the picture upright', () => {
    const placed = cutoutPlacement([32, 48], [16, 46], 128);
    expect(placed.width).toBeCloseTo(32 / 128);
    expect(placed.offsetX).toBeCloseTo(0);
    expect(placed.offsetY).toBeGreaterThan(0);
    expect(placed.height).toBeGreaterThan(48 / 128);
  });

  it('leaves a center pivot unshifted', () => {
    const placed = cutoutPlacement([32, 48], [16, 24], 128);
    expect(placed.offsetX).toBeCloseTo(0);
    expect(placed.offsetY).toBeCloseTo(0);
  });
});

it('ignores transparent sprite pixels and caches alpha sampling for later hits', async () => {
  const { opaqueTexel } = await import('./cutout');
  const { vi } = await import('vitest');
  const drawImage = vi.fn();
  const getImageData = vi.fn(() => ({ data: new Uint8ClampedArray([0, 0, 0, 0, 1, 1, 1, 255]) }));
  vi.stubGlobal('document', { createElement: () => ({ getContext: () => ({ drawImage, getImageData }) }) });
  const image = { width: 2, height: 1 } as HTMLImageElement;
  try {
    expect(opaqueTexel(image, 0.1, 0.5)).toBe(false);
    expect(opaqueTexel(image, 0.9, 0.5)).toBe(true);
    expect(opaqueTexel(image, 0.1, 0.5)).toBe(false);
    expect(drawImage).toHaveBeenCalledOnce();
    expect(getImageData).toHaveBeenCalledOnce();
  } finally { vi.unstubAllGlobals(); }
});
