import { beforeEach, describe, expect, it } from 'vitest';

import type { FieldDef } from '../net/bindings/FieldDef.ts';
import type { SystemDef } from '../net/bindings/SystemDef.ts';
import { table } from '../table.svelte';
import { designer } from './designer.svelte';
import { emptySheets, onSheet } from './sheets';

function field(id: string, name: string): FieldDef {
  return {
    id,
    name,
    kind: 'number',
    min: 0,
    max: null,
    maxFrom: null,
    default: 0,
    textDefault: '',
    look: 'boxes',
    options: [],
    steps: [],
    pool: null,
    formula: '',
    hint: '',
  };
}

function sample(): SystemDef {
  return {
    id: 'daggerheart',
    name: 'Daggerheart',
    fields: [field('hp', 'HP')],
    dieTags: [],
    actions: [],
    sheets: emptySheets(),
    grid: null,
  };
}

beforeEach(() => {
  designer.draft = null;
  designer.notice = null;
  designer.loadedFromCampaign = false;
  table.systems = [];
});

describe('designer.load', () => {
  it('copies a reactive system and says it loaded', () => {
    const source = $state(sample());
    table.systems = [source];
    designer.load(source);
    expect(designer.draft).toEqual(sample());
    expect(designer.notice?.tone).toBe('ok');
    expect(designer.view).toBe('overview');
    designer.place('pc', 'hp');
    expect(designer.draft && onSheet(designer.draft.sheets, 'pc', 'hp')).toBe(true);
  });

  it('duplicates into a new id and revert does not throw', () => {
    const source = $state(sample());
    table.systems = [source];
    designer.load(source);
    designer.duplicate();
    expect(designer.draft?.id).not.toBe('daggerheart');
    expect(designer.notice).toBeNull();
    expect(() => designer.revert()).not.toThrow();
    expect(designer.draft).toBeNull();
  });
});
