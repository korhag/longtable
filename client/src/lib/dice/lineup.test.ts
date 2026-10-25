import { describe, expect, it } from 'vitest';

import { lineUpSlots } from './lineup';

describe('lineup slots', () => {
  it('keeps every die at least a spacing apart', () => {
    const spacing = 1.35;
    const slots = lineUpSlots(8, 6, spacing);
    expect(slots).toHaveLength(8);
    for (let i = 0; i < slots.length; i += 1) {
      for (let j = i + 1; j < slots.length; j += 1) {
        const a = slots[i]!;
        const b = slots[j]!;
        const distance = Math.hypot(a.x - b.x, a.z - b.z);
        expect(distance).toBeGreaterThanOrEqual(spacing - 1e-6);
      }
    }
  });
});
