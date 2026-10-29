import { describe, expect, it } from 'vitest';

import { ADVERSARIES, ANCESTRIES, CLASSES, COMMUNITIES, DOMAIN_CARDS, JAGGED_KNIFE_BANDIT, RULES, cardsFor, diceOf, ruleHtml } from './catalog';

describe('the Daggerheart book', () => {
  it('loads every class, card, adversary, and rule chapter', () => {
    expect(CLASSES).toHaveLength(13);
    expect(CLASSES.flatMap((item) => item.subclasses)).toHaveLength(26);
    expect(DOMAIN_CARDS).toHaveLength(210);
    expect(ADVERSARIES).toHaveLength(264);
    expect(RULES).toHaveLength(17);
    expect(ANCESTRIES.length).toBeGreaterThanOrEqual(18);
    expect(COMMUNITIES.length).toBeGreaterThanOrEqual(9);
  });

  it('offers real level 1 cards for each class', () => {
    for (const item of CLASSES) {
      const cards = cardsFor(item.id);
      expect(cards.length, item.name).toBeGreaterThanOrEqual(2);
      expect(cards.every((card) => card.level === 1 && card.text.length > 0)).toBe(true);
      expect(item.features.length).toBeGreaterThan(0);
      expect(item.hope).toContain('\n');
    }
  });

  it('keeps the Jagged Knife Bandit and a flat minion damage roll', () => {
    expect(JAGGED_KNIFE_BANDIT.difficulty).toBe(12);
    expect(JAGGED_KNIFE_BANDIT.attack).toBe(1);
    expect(diceOf(JAGGED_KNIFE_BANDIT.damage)).toBe('1d8+1');
    const lackey = ADVERSARIES.find((item) => item.name === 'Jagged Knife Lackey');
    expect(diceOf(lackey?.damage ?? '')).toBe('flat:2');
    expect(lackey?.major).toBe(99);
  });

  it('renders rule headings and emphasis without raw markers', () => {
    const html = ruleHtml('**Welcome,** *adventure.*\n\n## WHAT IS THIS?\n\n- Look it up.');
    expect(html).toContain('<strong>Welcome,</strong>');
    expect(html).toContain('<em>adventure.</em>');
    expect(html).toContain('<h2>WHAT IS THIS?</h2>');
    expect(html).not.toContain('*');
  });

  it('renders a markdown table as a table', () => {
    const html = ruleHtml('| Name | Damage |\n| --- | --- |\n| **Cleaver** | d8 phy |\n| **Sickle** | ***Quick:*** mark a Stress |');
    expect(html).toContain('<table>');
    expect(html).toContain('<th>Name</th>');
    expect(html).toContain('<td><strong>Cleaver</strong></td>');
    expect(html).toContain('<td><strong><em>Quick:</em></strong> mark a Stress</td>');
    expect(html).not.toContain('| --- |');
  });
});