import { describe, expect, it } from 'vitest';

import type { Character } from '../../lib/net/bindings/Character.ts';
import { attackFormula } from './attack';

function character(kind: Character['kind'], numbers: Record<string, number>, primary = ''): Character {
  return {
    id: 'c',
    name: 'Ada',
    kind,
    ownerId: null,
    systemId: 'daggerheart',
    numbers,
    texts: { primary },
  };
}

describe('attack formula', () => {
  it('uses the trait named on the primary weapon', () => {
    const sheet = character('pc', { instinct: 2 }, 'Dualstaff · Instinct · Far · 1d6+3 mag');
    expect(attackFormula(sheet)).toBe('1d12h+1d12f+2');
  });

  it('rolls duality with no bonus when the weapon names no trait', () => {
    expect(attackFormula(character('pc', { agility: 1 }, 'Torch'))).toBe('1d12h+1d12f');
  });

  it('keeps a negative trait modifier', () => {
    expect(attackFormula(character('pc', { strength: -1 }, 'Maul · Strength · Melee · 1d8'))).toBe('1d12h+1d12f-1');
  });

  it('rolls an adversary attack modifier', () => {
    expect(attackFormula(character('adversary', { attack: 1 }))).toBe('1d20+1');
    expect(attackFormula(character('adversary', {}))).toBe('1d20');
    expect(attackFormula(character('adversary', { attack: -1 }))).toBe('1d20-1');
  });
});
