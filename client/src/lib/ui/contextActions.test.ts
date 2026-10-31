import { expect, it } from 'vitest';
import type { Character } from '../net/bindings/Character';
import type { TokenView } from '../net/bindings/TokenView';
import type { SystemDef } from '../net/bindings/SystemDef';
import { contextActions } from './contextActions';
const selected = { id: 'hero', ownerId: 'player', characterId: 'hero-sheet', name: 'Hero' } as TokenView;
const foe = { id: 'bandit', ownerId: null, characterId: 'foe-sheet', name: 'Bandit' } as TokenView;
const actor = { id: 'hero-sheet', kind: 'pc', ownerId: 'player', name: 'Hero' } as Character;
const system = { id: 'daggerheart', actions: [{ id: 'agility', name: 'Agility', formula: '1d12h+1d12f' }], sheets: { pc: { toolbar: { actions: ['agility'] } } } } as unknown as SystemDef;
const base = { clicked: foe, selected, actor, target: foe, system, gm: false, seat: 'player', readOnly: false, hasPoint: true, hasClickedSheet: false };
it('offers attacks for the controlled actor against the clicked opponent, without GM actions', () => {
  const ids = contextActions(base).map(action => action.id);
  expect(ids).toContain('attack');
  expect(ids).toContain('target');
  expect(ids).not.toContain('select');
  expect(ids).not.toContain('hide');
  expect(contextActions({ ...base, seat: 'spectator' }).map(action => action.id)).not.toContain('attack');
});
it('uses the selected character’s configured rule actions and supports neutral systems', () => {
  expect(contextActions({ ...base, clicked: selected }).map(action => action.id)).toContain('rule:agility');
  const neutral = contextActions({ ...base, system: null }).map(action => action.id);
  expect(neutral).toContain('target');
  expect(neutral).not.toContain('attack');
});
it('offers pathfinding movement and reserves teleport for a GM outside player preview', () => {
  const options = { ...base, clicked: null };
  expect(contextActions(options).map(action => action.id)).toEqual(['move', 'center-point']);
  expect(contextActions({ ...options, gm: true }).map(action => action.id)).toContain('teleport');
  expect(contextActions({ ...options, gm: true, readOnly: true }).map(action => action.id)).toEqual(['center-point']);
});
