import { describe, expect, it } from 'vitest';

import { emptySheets, placeOn, removeFrom, thingId } from './sheets';

describe('thing ids', () => {
  it('builds an id from the name and avoids ones already taken', () => {
    expect(thingId('Hit Points', [])).toBe('hitpoints');
    expect(thingId('HP', [])).toBe('hp');
    expect(thingId('', [])).toBe('thing');
    expect(thingId('2nd wind', [])).toBe('ndwind');
    expect(thingId('Hit Points', ['hitpoints'])).toBe('hitpoints2');
    expect(thingId('A very long name that will not fit in a field id', [])).toHaveLength(24);
  });
});

describe('placing a thing on a sheet', () => {
  it('adds the thing to the last section and remove takes it off the toolbar too', () => {
    const placed = placeOn(emptySheets(), 'pc', 'hp');
    expect(placed.pc.sections).toEqual([{ title: 'Sheet', columns: 2, fields: ['hp'] }]);
    const again = placeOn(placed, 'pc', 'hp');
    expect(again.pc.sections[0]?.fields).toEqual(['hp']);
    const withBar = placeOn(placed, 'pc', 'stress');
    withBar.pc.toolbar.resources = ['hp', 'stress'];
    const removed = removeFrom(withBar, 'pc', 'hp');
    expect(removed.pc.sections[0]?.fields).toEqual(['stress']);
    expect(removed.pc.toolbar.resources).toEqual(['stress']);
    expect(withBar.pc.sections[0]?.fields).toEqual(['hp', 'stress']);
  });
});
