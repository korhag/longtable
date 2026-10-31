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
  if (blow.pendingArmor) return `Awaiting damage choice · ${blow.marks} HP before armor`;
  if (blow.marks === 0) return 'No HP marked';
  const applied = blow.appliedMarks;
  return `${applied} HP marked${blow.armorUsed ? ` · Armor used (${blow.marks} → ${Math.max(0, blow.marks - 1)} HP)` : applied < blow.marks ? ' (HP limit reached)' : ''}${blow.direct ? ' · Direct damage' : ''}`;
}

export function severityName(marks: number): string {
  return ['None', 'Minor', 'Major', 'Severe'][marks] ?? `${marks} HP`;
}

export function damageSteps(major: number | null, severe: number | null) {
  return [
    { name: 'None', marks: 0, range: '0' },
    { name: 'Minor', marks: 1, range: major == null ? 'Below Major' : `1–${Math.max(1, major - 1)}` },
    { name: 'Major', marks: 2, range: major == null || severe == null ? 'Major' : `${major}–${severe - 1}` },
    { name: 'Severe', marks: 3, range: severe == null ? 'Severe' : `${severe}+` },
  ];
}
