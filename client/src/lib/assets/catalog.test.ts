import { describe, expect, it } from 'vitest';

import { cachedImage, imageDecodeCount } from './catalog';

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
