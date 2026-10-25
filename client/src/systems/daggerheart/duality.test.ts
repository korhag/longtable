import { describe, expect, it } from 'vitest';

import { dualityFormula, type DualityConfig } from './duality';

const base: DualityConfig = { hope: 12, fear: 12, edge: 'none' };

describe('duality formula', () => {
  it('builds each setup the server can parse', () => {
    expect(dualityFormula(base)).toBe('1d12h+1d12f');
    expect(dualityFormula({ ...base, edge: 'advantage' })).toBe('1d12h+1d12f+1d6');
    expect(dualityFormula({ ...base, edge: 'disadvantage' })).toBe('1d12h+1d12f-1d6');
    expect(dualityFormula({ hope: 20, fear: 12, edge: 'none' })).toBe('1d20h+1d12f');
    expect(dualityFormula({ hope: 12, fear: 20, edge: 'advantage' })).toBe('1d12h+1d20f+1d6');
  });
});
