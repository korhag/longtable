import { beforeEach, expect, it, vi } from 'vitest';
import { attackCharacter } from './targetedAttack';
import { send } from '../net/socket';
import { table } from '../table.svelte';
import { rollSetup } from './rollSetup.svelte';
import type { Character } from '../net/bindings/Character';
import type { TokenView } from '../net/bindings/TokenView';

vi.mock('../net/socket', () => ({ send: vi.fn() }));
vi.mock('../table.svelte', () => ({ table: { characters: [], tokens: [], selectedTokenId: null, targetTokenId: null, error: null } }));
vi.mock('./rollSetup.svelte', () => ({ rollSetup: { plus: 2, minus: 1, attackEdge: 'advantage' } }));

beforeEach(() => {
  vi.mocked(send).mockClear();
  table.characters = [{ id: 'hero', kind: 'pc' }, { id: 'foe', kind: 'adversary' }] as Character[];
  table.tokens = [{ id: 'hero-body', characterId: 'hero' }, { id: 'foe-body', characterId: 'foe' }, { id: 'second-foe', characterId: 'foe' }] as TokenView[];
  table.selectedTokenId = 'hero-body'; table.targetTokenId = 'second-foe'; table.error = null;
  rollSetup.attackEdge = 'advantage';
});

it('sends the exact targeted instance and PC circumstances through the rules attack endpoint', () => {
  attackCharacter('hero');
  expect(send).toHaveBeenCalledWith({ type: 'attack', attackerId: 'hero', targetId: 'foe', attackerTokenId: 'hero-body', targetTokenId: 'second-foe', edge: 'advantage', plus: 2, minus: 1 });
});

it('keeps adversary attacks separate from the PC Duality edge and resolves the attacking sheet’s figure', () => {
  table.targetTokenId = 'hero-body';
  attackCharacter('foe');
  expect(send).toHaveBeenCalledWith(expect.objectContaining({ attackerId: 'foe', attackerTokenId: 'foe-body', targetId: 'hero', edge: 'none' }));
});

it('requires another linked character as the target and sends no stray generic damage roll', () => {
  table.targetTokenId = 'hero-body'; attackCharacter('hero');
  expect(send).not.toHaveBeenCalled();
  expect(table.error).toContain('target');
  table.targetTokenId = null; attackCharacter('hero');
  expect(send).not.toHaveBeenCalled();
});
