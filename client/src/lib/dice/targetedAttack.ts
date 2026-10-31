import { send } from '../net/socket';
import { table } from '../table.svelte';
import { rollSetup } from './rollSetup.svelte';

/** All attack buttons use the same targeted rules flow, never a generic d20. */
export function attackCharacter(characterId: string, targetTokenId = table.targetTokenId) {
  const actor = table.characters.find(character => character.id === characterId);
  const target = table.tokens.find(token => token.id === targetTokenId);
  if (!actor || !target?.characterId || target.characterId === actor.id) {
    table.error = 'Pick another character as the target.';
    return;
  }
  const selected = table.tokens.find(token => token.id === table.selectedTokenId && token.characterId === actor.id)
    ?? table.tokens.find(token => token.characterId === actor.id);
  send({ type: 'attack', attackerId: actor.id, targetId: target.characterId,
    attackerTokenId: selected?.id, targetTokenId: target.id,
    edge: actor.kind === 'pc' ? rollSetup.attackEdge : 'none', plus: rollSetup.plus, minus: rollSetup.minus });
}
