import { describe, expect, it } from 'vitest';

import { bitAt, decodeBits } from './bits';

describe('fog bits', () => {
  it('reads the high bit of the first byte as cell zero', () => {
    // 0b10000001 is the byte for cells 0 and 7.
    const bytes = decodeBits(btoa(String.fromCharCode(0b10000001)));
    expect(bitAt(bytes, 0)).toBe(true);
    expect(bitAt(bytes, 1)).toBe(false);
    expect(bitAt(bytes, 7)).toBe(true);
    expect(bitAt(bytes, 8)).toBe(false);
  });
});
