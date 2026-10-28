// ------------------------------------------------------------
// Module: draft
// Purpose: Turn the creation choices into the numbers and text
//          a character already stores. The server does not learn
//          a new message for this.
// ------------------------------------------------------------

import type { Character } from '../../lib/net/bindings/Character.ts';
import {
  ARMORS,
  CONNECTION_PROMPTS,
  STARTING_MODIFIERS,
  TRAITS,
  armorById,
  cardsFor,
  classById,
  diceOf,
  weaponById,
  type TraitId,
} from './catalog';

export interface CardChoice {
  id: string;
  domain: string;
  name: string;
  taken: boolean;
}

export interface CreationDraft {
  classId: string;
  subclassId: string;
  ancestry: string;
  community: string;
  name: string;
  pronouns: string;
  description: string;
  traits: Record<TraitId, number | null>;
  weaponId: string;
  secondaryId: string;
  armorId: string;
  potion: 'health' | 'stamina';
  keepsake: string;
  background: [string, string, string];
  experiences: [string, string];
  cards: CardChoice[];
  connections: [string, string, string];
  place: boolean;
}

export interface CharacterPayload {
  name: string;
  numbers: Record<string, number>;
  texts: Record<string, string>;
}

export function emptyTraits(): Record<TraitId, number | null> {
  return { agility: null, strength: null, finesse: null, instinct: null, presence: null, knowledge: null };
}

export function emptyDraft(): CreationDraft {
  return {
    classId: '',
    subclassId: '',
    ancestry: '',
    community: '',
    name: '',
    pronouns: '',
    description: '',
    traits: emptyTraits(),
    weaponId: '',
    secondaryId: 'dagger',
    armorId: '',
    potion: 'health',
    keepsake: '',
    background: ['', '', ''],
    experiences: ['', ''],
    cards: [],
    connections: ['', '', ''],
    place: true,
  };
}

export function freshCards(classId: string): CardChoice[] {
  return cardsFor(classId).map((card) => ({ id: card.id, domain: card.domain, name: card.name, taken: false }));
}

export function applyClass(draft: CreationDraft, classId: string): CreationDraft {
  const chosen = classById(classId);
  return {
    ...draft,
    classId,
    subclassId: '',
    weaponId: chosen?.weapon ?? '',
    armorId: chosen?.armor ?? '',
    keepsake: chosen?.keepsakes[0] ?? '',
    cards: freshCards(classId),
    background: ['', '', ''],
  };
}

export function useSuggestion(draft: CreationDraft): CreationDraft {
  const chosen = classById(draft.classId);
  if (!chosen) return draft;
  return { ...draft, traits: { ...chosen.suggested } };
}

export function remainingModifiers(traits: Record<TraitId, number | null>, held: number | null): number[] {
  const bag: number[] = [...STARTING_MODIFIERS];
  for (const trait of TRAITS) {
    const value = traits[trait.id];
    if (value === null) continue;
    const index = bag.indexOf(value);
    if (index >= 0) bag.splice(index, 1);
  }
  if (held !== null) {
    const index = bag.indexOf(held);
    if (index >= 0) bag.splice(index, 1);
  }
  return bag;
}

export function traitsReady(traits: Record<TraitId, number | null>): boolean {
  return remainingModifiers(traits, null).length === 0 && TRAITS.every((trait) => traits[trait.id] !== null);
}

export function evasionOf(draft: CreationDraft): number {
  const chosen = classById(draft.classId);
  const armor = armorById(draft.armorId);
  return (chosen?.evasion ?? 10) + (armor?.evasion ?? 0);
}

export function thresholdsOf(draft: CreationDraft, level = 1): { minor: number; major: number; severe: number } {
  const armor = armorById(draft.armorId) ?? ARMORS[1];
  return { minor: 1, major: (armor?.major ?? 6) + level, severe: (armor?.severe ?? 13) + level };
}

export function takenCards(draft: CreationDraft): CardChoice[] {
  return draft.cards.filter((card) => card.taken);
}

function lines(title: string, body: string): string {
  const text = body.trim();
  return text ? `${title}\n${text}` : '';
}

export function payload(draft: CreationDraft): CharacterPayload {
  const chosen = classById(draft.classId);
  const weapon = weaponById(draft.weaponId);
  const offhand = weapon?.hands === 2 ? undefined : weaponById(draft.secondaryId);
  const armor = armorById(draft.armorId);
  const gates = thresholdsOf(draft);
  const potion = draft.potion === 'health'
    ? 'Minor Health Potion (clear 1d4 Hit Points)'
    : 'Minor Stamina Potion (clear 1d4 Stress)';
  const inventory = [
    'Torch',
    '50 feet of rope',
    'Basic supplies',
    potion,
    draft.keepsake.trim(),
  ].filter(Boolean);
  const weaponLine = (item: NonNullable<typeof weapon>) => `${item.name} · ${label(item.trait)} · ${item.range} · ${item.damage}`;
  const background = chosen
    ? chosen.questions.map((question, index) => lines(question, draft.background[index] ?? '')).filter(Boolean).join('\n\n')
    : '';
  const connections = CONNECTION_PROMPTS
    .map((question, index) => lines(question, draft.connections[index] ?? ''))
    .filter(Boolean)
    .join('\n\n');
  const experiences = draft.experiences.map((name) => `${name.trim()} +2`).filter((line) => line !== '+2');
  const name = draft.name.trim().slice(0, 24) || 'Adventurer';

  return {
    name,
    numbers: {
      level: 1,
      proficiency: 1,
      agility: draft.traits.agility ?? 0,
      strength: draft.traits.strength ?? 0,
      finesse: draft.traits.finesse ?? 0,
      instinct: draft.traits.instinct ?? 0,
      presence: draft.traits.presence ?? 0,
      knowledge: draft.traits.knowledge ?? 0,
      evasion: evasionOf(draft),
      armorscore: armor?.score ?? 0,
      hpmax: chosen?.hp ?? 6,
      hp: 0,
      stressmax: 6,
      stress: 0,
      hope: 2,
      armor: 0,
      gold: 1,
      minor: gates.minor,
      major: gates.major,
      severe: gates.severe,
    },
    texts: {
      class: chosen?.name ?? '',
      subclass: chosen?.subclasses.find((item) => item.id === draft.subclassId)?.name ?? '',
      ancestry: draft.ancestry,
      community: draft.community,
      pronouns: draft.pronouns.trim(),
      description: draft.description.trim(),
      domains: chosen ? chosen.domains.join(' & ') : '',
      feature: (chosen?.features ?? []).join('\n'),
      experiences: experiences.join('\n'),
      primary: weapon ? weaponLine(weapon) : '',
      secondary: offhand ? weaponLine(offhand) : '',
      armorname: armor ? `${armor.name} · ${armor.score} · ${gates.major}/${gates.severe}` : '',
      inventory: inventory.join('\n'),
      background,
      connections,
      cards: takenCards(draft).map((card) => `${card.domain} — ${card.name.trim() || card.domain}`).join('\n'),
      damage: weapon ? diceOf(weapon.damage) : '1d8',
      notes: '',
    },
  };
}

export function previewCharacter(draft: CreationDraft): Character {
  const body = payload(draft);
  return {
    id: 'preview',
    name: body.name,
    kind: 'pc',
    ownerId: null,
    systemId: 'daggerheart',
    numbers: body.numbers,
    texts: body.texts,
  };
}

function label(id: TraitId): string {
  return TRAITS.find((trait) => trait.id === id)?.name ?? id;
}

export interface AdversaryDraft {
  name: string;
  type: string;
  tier: number;
  description: string;
  motives: string;
  difficulty: number;
  major: number;
  severe: number;
  hp: number;
  stress: number;
  attack: number;
  attackName: string;
  attackRange: string;
  damage: string;
  experience: string;
  features: string;
  place: boolean;
}

export function adversaryPayload(draft: AdversaryDraft): CharacterPayload {
  const name = draft.name.trim().slice(0, 24) || 'Adversary';
  return {
    name,
    numbers: {
      tier: clamp(draft.tier, 1, 4),
      difficulty: clamp(draft.difficulty, 1, 30),
      attack: clamp(draft.attack, -5, 12),
      hpmax: clamp(draft.hp, 1, 20),
      hp: 0,
      stressmax: clamp(draft.stress, 0, 20),
      stress: 0,
      minor: 1,
      major: clamp(draft.major, 1, 40),
      severe: clamp(draft.severe, 1, 60),
    },
    texts: {
      advtype: draft.type || 'Standard',
      description: draft.description.trim(),
      motives: draft.motives.trim(),
      attackname: draft.attackName.trim() || 'Attack',
      attackrange: draft.attackRange.trim() || 'Melee',
      atkdamage: diceOf(draft.damage) || '1d8',
      advexp: draft.experience.trim(),
      features: draft.features.trim(),
    },
  };
}

export function previewAdversary(draft: AdversaryDraft): Character {
  const body = adversaryPayload(draft);
  return {
    id: 'preview',
    name: body.name,
    kind: 'adversary',
    ownerId: null,
    systemId: 'daggerheart',
    numbers: body.numbers,
    texts: body.texts,
  };
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}
