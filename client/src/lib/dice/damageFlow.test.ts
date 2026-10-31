import { describe, expect, it } from 'vitest';
import type { Character } from '../net/bindings/Character';
import type { Contest } from '../net/bindings/Contest';
import type { RollEvent } from '../net/bindings/RollEvent';
import type { Blow } from '../net/bindings/Blow';
import { damageApplication, damageThresholds, damageSteps, mayRollDamage } from './damageFlow';

const hero: Character = { id: 'hero', name: 'Hero', kind: 'pc', ownerId: 'alice', systemId: 'daggerheart', numbers: {}, texts: {} };
const contest: Contest = {
  attackerId: 'hero', attackerName: 'Hero', targetId: 'bandit', targetName: 'Bandit',
  defenseName: 'Difficulty', defense: 12, verdict: 'hit', blow: null, damageRolled: false, attackRollId: null,
};
const hit = { id: 'attack', rollerId: 'alice', contest } as RollEvent;
const major: Blow = { step: 'Major', marks: 2, appliedMarks: 2, pendingArmor: false, direct: false, armorUsed: false, majorThreshold: 9, severeThreshold: 18 };

describe('manual Daggerheart damage', () => {
  it('allows the current owner or GM after a hit, but never spectators or GM player preview', () => {
    expect(mayRollDamage(hit, 'daggerheart', [hero], 'player', 'alice', null)).toBe(true);
    expect(mayRollDamage(hit, 'daggerheart', [hero], 'gm', 'gm', null)).toBe(true);
    expect(mayRollDamage(hit, 'daggerheart', [hero], 'player', 'bob', null)).toBe(false);
    expect(mayRollDamage(hit, 'daggerheart', [hero], 'gm', 'gm', 'alice')).toBe(false);
    expect(mayRollDamage(hit, 'daggerheart', [{ ...hero, ownerId: 'bob' }], 'player', 'alice', null)).toBe(false);
  });

  it('rejects misses, unrelated systems and damage already claimed in the latest snapshot', () => {
    expect(mayRollDamage({ ...hit, contest: { ...contest, verdict: 'miss' } }, 'daggerheart', [hero], 'player', 'alice', null)).toBe(false);
    expect(mayRollDamage(hit, 'another-system', [hero], 'player', 'alice', null)).toBe(false);
    expect(mayRollDamage(hit, 'daggerheart', [hero], 'player', 'alice', null, { ...contest, damageRolled: true })).toBe(false);
    expect(mayRollDamage({ ...hit, contest: { ...contest, damageRolled: true } }, 'daggerheart', [hero], 'player', 'alice', null)).toBe(false);
    expect(mayRollDamage({ ...hit, contest: { ...contest, blow: major } }, 'daggerheart', [hero], 'player', 'alice', null)).toBe(false);
  });

  it('explains configured threshold values and the HP actually marked', () => {
    expect(damageThresholds(major)).toBe('Major 9+ · Severe 18+');
    expect(damageThresholds({ ...major, majorThreshold: null, severeThreshold: null })).toBe('');
    expect(damageApplication(major)).toBe('2 HP marked');
    expect(damageApplication({ ...major, step: 'Severe', marks: 3, appliedMarks: 1 })).toBe('1 HP marked (HP limit reached)');
    expect(damageApplication({ ...major, appliedMarks: 0, pendingArmor: true })).toBe('Awaiting damage choice · 2 HP before armor');
    expect(damageApplication({ ...major, step: 'None', marks: 0, appliedMarks: 0 })).toBe('No HP marked');
  });
  it('shows the four severity positions and an armor reduction down to no HP', () => {
    expect(damageSteps(9, 18).map(step => step.range)).toEqual(['0', '1–8', '9–17', '18+']);
    expect(damageApplication({ ...major, armorUsed: true, appliedMarks: 1 })).toBe('1 HP marked · Armor used (2 → 1 HP)');
    expect(damageApplication({ ...major, step: 'Minor', marks: 1, armorUsed: true, appliedMarks: 0 })).toBe('0 HP marked · Armor used (1 → 0 HP)');
    expect(damageApplication({ ...major, direct: true })).toBe('2 HP marked · Direct damage');
  });

});
