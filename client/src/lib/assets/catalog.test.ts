import { describe, expect, it } from 'vitest';

import { cachedImage, imageDecodeCount, seamCells } from './catalog';

describe('cached pictures', () => {
  it('decodes one url once', () => {
    const before = imageDecodeCount();
    const url = 'http://127.0.0.1:1/longtable-tile-once.png';
    const first = cachedImage(url);
    const second = cachedImage(url);
    expect(second).toBe(first);
    expect(imageDecodeCount()).toBe(before + 1);
  });
});

describe('tile seams', () => {
  it('alternates the inspected tile with its neighbor', () => {
    expect(seamCells('grass', 'dirt')).toEqual(['grass', 'dirt', 'grass', 'dirt', 'grass', 'dirt', 'grass', 'dirt', 'grass']);
  });
});
