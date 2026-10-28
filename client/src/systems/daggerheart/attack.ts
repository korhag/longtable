// ------------------------------------------------------------
// Module: attack
// Purpose: The formula a targeted attack sends. A player
//          character uses the trait written on their primary
//          weapon. An adversary uses their attack modifier.
// ------------------------------------------------------------

import type { Character } from '../../lib/net/bindings/Character.ts';
import { TRAITS } from './catalog';

export function attackFormula(character: Character): string {
  if (character.kind === 'pc') {
    const trait = traitId(character.texts.primary ?? '');
    const mod = trait ? (character.numbers[trait] ?? 0) : 0;
    return `1d12h+1d12f${signed(mod)}`;
  }
  return `1d20${signed(character.numbers.attack ?? 0)}`;
}

function signed(mod: number): string {
  if (mod > 0) return `+${mod}`;
  if (mod < 0) return `${mod}`;
  return '';
}

function traitId(primary: string): string | null {
  const parts = primary.split('·').map((part) => part.trim().toLowerCase());
  const trait = TRAITS.find((item) => parts.includes(item.name.toLowerCase()));
  return trait?.id ?? null;
}
