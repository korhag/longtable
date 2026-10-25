import { describe, expect, it } from 'vitest';

import { ClockSync, offsetFromPing } from './clock';

describe('clock sync', () => {
  it('removes half the round trip from the offset', () => {
    // Sent at local 1_000, server said 5_000, reply arrived at local 1_200.
    // The server was sampled halfway, at local 1_100, so the offset is 3_900.
    expect(offsetFromPing(1_000, 5_000, 1_200)).toBe(3_900);
  });

  it('smooths later samples instead of jumping', () => {
    const clock = new ClockSync();
    clock.push(100);
    clock.push(200);
    expect(clock.offset).toBeCloseTo(120);
    expect(clock.serverNow(1_000)).toBeCloseTo(1_120);
  });
});
