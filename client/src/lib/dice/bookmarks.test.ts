import { describe, expect, it } from 'vitest';

import { bookmarkMatches, removeBookmark, upsertBookmark, type DiceBookmark } from './bookmarks';

describe('bookmarks', () => {
  const attack: DiceBookmark = { label: 'Attack', formula: '1d20', plus: 3, minus: 0 };

  it('overwrites the same label in any letter case and can drop one', () => {
    const saved = upsertBookmark([attack], { label: 'attack', formula: '2d6', plus: 0, minus: 1 });
    expect(saved).toEqual([{ label: 'attack', formula: '2d6', plus: 0, minus: 1 }]);
    expect(removeBookmark(saved, 'Attack')).toEqual([]);
  });

  it('matches only an identical setup', () => {
    expect(bookmarkMatches(attack, { label: 'ATTACK', formula: '1d20', plus: 3, minus: 0 })).toBe(true);
    expect(bookmarkMatches(attack, { label: 'Attack', formula: '1d20', plus: 4, minus: 0 })).toBe(false);
  });
});
