import { describe, expect, it } from 'vitest';

import type { FieldDef } from '../net/bindings/FieldDef.ts';
import type { SystemDef } from '../net/bindings/SystemDef.ts';
import { describeFormula, describeThing } from './describe';
import { emptySheets } from './sheets';

function field(partial: Partial<FieldDef> & Pick<FieldDef, 'id' | 'name' | 'kind'>): FieldDef {
  return {
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
    ...partial,
  };
}

function system(fields: FieldDef[], extra: Partial<SystemDef> = {}): SystemDef {
  return {
    id: 'sample',
    name: 'Sample',
    fields,
    dieTags: [
      { letter: 'h', id: 'hope', name: 'Hope', body: '#d4a656', ink: '#1c140e' },
      { letter: 'f', id: 'fear', name: 'Fear', body: '#3c2158', ink: '#f4ecdf' },
    ],
    actions: [],
    sheets: emptySheets(),
    grid: null,
    ...extra,
  };
}

describe('describeThing', () => {
  it('says where a pool takes its maximum', () => {
    const hpmax = field({ id: 'hpmax', name: 'HP max', kind: 'number', default: 6 });
    const hp = field({ id: 'hp', name: 'HP', kind: 'pool', look: 'boxes', max: 6, maxFrom: 'hpmax' });
    expect(describeThing(hp, system([hpmax, hp]))).toBe('A pool of boxes; its maximum comes from HP max (6).');
  });

  it('lists threshold steps and the pool they mark', () => {
    const hp = field({ id: 'hp', name: 'HP', kind: 'pool' });
    const thresholds = field({
      id: 'thresholds',
      name: 'Thresholds',
      kind: 'thresholds',
      pool: 'hp',
      steps: [
        { id: 'minor', name: 'Minor', default: 1, marks: 1 },
        { id: 'major', name: 'Major', default: 7, marks: 2 },
        { id: 'severe', name: 'Severe', default: 13, marks: 3 },
      ],
    });
    expect(describeThing(thresholds, system([hp, thresholds]))).toBe(
      'Minor at 1 marks 1 HP, Major at 7 marks 2 HP, Severe at 13 marks 3 HP.',
    );
  });
});

describe('describeFormula', () => {
  it('names die tags and thing references', () => {
    const agility = field({ id: 'agility', name: 'Agility', kind: 'number' });
    expect(describeFormula('1d12h+1d12f+@agility', system([agility]))).toBe('Hope d12 + Fear d12 + Agility');
  });
});
