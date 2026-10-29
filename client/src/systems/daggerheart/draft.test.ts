import { describe, expect, it } from 'vitest';

import { CLASSES, STARTING_MODIFIERS, TRAITS } from './catalog';
import { applyClass, emptyDraft, payload, remainingModifiers, traitsReady, useSuggestion } from './draft';

describe('character creation', () => {
  it('gives every class the starting modifier set', () => {
    const want = [...STARTING_MODIFIERS].sort((a, b) => a - b);
    for (const item of CLASSES) {
      const got = TRAITS.map((trait) => item.suggested[trait.id]).sort((a, b) => a - b);
      expect(got, item.name).toEqual(want);
      expect(item.subclasses).toHaveLength(2);
      expect(item.domains).toHaveLength(2);
      expect(item.hp).toBeGreaterThan(0);
    }
  });

  it('builds a level 1 sorcerer with hope, a weapon die, and two cards', () => {
    let draft = applyClass(emptyDraft(), 'sorcerer');
    draft = useSuggestion(draft);
    draft.subclassId = 'primal';
    draft.ancestry = 'Elf';
    draft.community = 'Loreborne';
    draft.name = 'Marlowe Fairwind';
    draft.gender = 'Female';
    draft.experiences = ['Royal Mage', 'Not On My Watch'];
    draft.cards = draft.cards.map((card, index) => ({ ...card, taken: index < 2, name: index === 0 ? 'Spark' : card.name }));
    expect(traitsReady(draft.traits)).toBe(true);
    expect(remainingModifiers(draft.traits, null)).toEqual([]);
    const body = payload(draft);
    expect(body.name).toBe('Marlowe Fairwind');
    expect(body.numbers.hope).toBe(2);
    expect(body.numbers.level).toBe(1);
    expect(body.numbers.evasion).toBe(11);
    expect(body.numbers.hpmax).toBe(6);
    expect(body.numbers.major).toBe(6);
    expect(body.texts.gender).toBe('Female');
    expect(body.texts.class).toBe('Sorcerer');
    expect(body.texts.subclass).toBe('Primal Origin');
    expect(body.texts.damage).toBe('1d6+3');
    expect(body.texts.experiences).toContain('Royal Mage +2');
    expect(body.texts.cards).toContain('Spark');
    expect(body.texts.cards).toContain(draft.cards[1]?.name ?? '');
    expect(body.texts.feature.length).toBeGreaterThan(40);
    expect(body.texts.inventory).toContain('Torch');
    expect(body.texts.secondary).toBe('');
  });
});
