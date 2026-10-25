import { describe, expect, it } from 'vitest';

import { applyModifiers, clampModifier } from './rollText';

describe('roll modifiers', () => {
  it('stays at zero or above and only appends a real modifier', () => {
    expect(clampModifier('-4')).toBe(0);
    expect(clampModifier('2.9')).toBe(2);
    expect(clampModifier('nope')).toBe(0);
    expect(applyModifiers('1d20', 0, 0)).toBe('1d20');
    expect(applyModifiers('1d20', 3, 0)).toBe('1d20+3');
    expect(applyModifiers('1d12h+1d12f', 0, 2)).toBe('1d12h+1d12f-2');
    expect(applyModifiers('1d8', 1, 4)).toBe('1d8+1-4');
    expect(applyModifiers('1d20+1d6', 2, 1)).not.toContain('+-');
  });
});
