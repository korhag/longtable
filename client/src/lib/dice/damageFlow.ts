import type { Blow } from '../net/bindings/Blow';
import type { Character } from '../net/bindings/Character';
import type { Contest } from '../net/bindings/Contest';
import type { RollEvent } from '../net/bindings/RollEvent';

/** UI affordance only: the server rechecks control, the hit, and one-time use. */
export function mayRollDamage(
  roll: RollEvent | null,
  systemId: string | null,
  characters: Character[],
  role: string,
  playerId: string | null,
  viewAs: string | null,
  recorded: Contest | null = null,
): boolean {
  const contest = roll?.contest;
  if (systemId !== 'daggerheart' || viewAs || !contest || contest.verdict !== 'hit' || contest.blow) return false;
  if (contest.damageRolled || recorded?.damageRolled) return false;
  const attacker = characters.find((character) => character.id === contest.attackerId);
  return !!attacker && (role === 'gm' || (!!playerId && attacker.ownerId === playerId));
}

export function damageThresholds(blow: Blow | null): string {
  if (!blow || blow.majorThreshold == null || blow.severeThreshold == null) return '';
  return `Major ${blow.majorThreshold}+ · Severe ${blow.severeThreshold}+`;
}

export function damageApplication(blow: Blow): string {
  if (blow.pendingArmor) return `Awaiting armor choice · ${blow.marks} HP before armor`;
  if (blow.marks === 0) return 'No HP marked';
  const applied = blow.appliedMarks;
  return `${applied} HP marked${applied < blow.marks ? ` (reduced from ${blow.marks})` : ''}`;
}
