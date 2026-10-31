import type { Character } from '../net/bindings/Character';
import type { CastAnimationKind } from './castAnimation';

/** Prefer the explicit Daggerheart weapon range; primary text covers other sheets. */
export function attackAnimation(character: Pick<Character, 'texts'> | undefined, castAsset?: string | null): CastAnimationKind {
  const range = character?.texts.attackrange || character?.texts.primary || '';
  if (!range && (castAsset === 'forest/elf' || castAsset === 'forest/bandit-archer')) return 'ranged';
  return /\b(far|ranged|close)\b/i.test(range) && !/\b(melee|very close)\b/i.test(range) ? 'ranged' : 'melee';
}
