// ------------------------------------------------------------
// Module: catalog
// Purpose: Starting gear, and the door to the SRD book. Classes,
//          domain cards, adversaries, ancestries, and rules are
//          loaded in srd.ts from the Daggerheart SRD.
// ------------------------------------------------------------

import {
  ADVERSARIES,
  ADVERSARY_TYPES,
  ANCESTRIES,
  CLASSES,
  COMMUNITIES,
  DOMAIN_CARDS,
  JAGGED_KNIFE_BANDIT,
  RULES,
  ancestryByName,
  communityByName,
  plain,
  ruleHtml,
} from './srd';

export {
  ADVERSARIES,
  ADVERSARY_TYPES,
  ANCESTRIES,
  CLASSES,
  COMMUNITIES,
  DOMAIN_CARDS,
  JAGGED_KNIFE_BANDIT,
  RULES,
  ancestryByName,
  communityByName,
  plain,
  ruleHtml,
};

export type { HeritageDef, RuleSection } from './srd';

export const TRAITS = [
  { id: 'agility', name: 'Agility', verbs: 'Sprint, Leap, Maneuver' },
  { id: 'strength', name: 'Strength', verbs: 'Lift, Smash, Grapple' },
  { id: 'finesse', name: 'Finesse', verbs: 'Control, Hide, Tinker' },
  { id: 'instinct', name: 'Instinct', verbs: 'Perceive, Sense, Navigate' },
  { id: 'presence', name: 'Presence', verbs: 'Charm, Perform, Deceive' },
  { id: 'knowledge', name: 'Knowledge', verbs: 'Recall, Analyze, Comprehend' },
] as const;

export type TraitId = (typeof TRAITS)[number]['id'];

export const STARTING_MODIFIERS = [2, 1, 1, 0, 0, -1] as const;

export const EXPERIENCE_EXAMPLES = [
  'Assassin', 'Blacksmith', 'Bodyguard', 'Bounty Hunter', 'Circus Performer', 'Con Artist', 'Field Medic',
  'Merchant', 'Noble', 'Pirate', 'Scholar', 'Sellsword', 'Soldier', 'Storyteller', 'Thief',
  'Battle-Hardened', 'Bookworm', 'Lone Wolf', 'Silver Tongue', 'Sticky Fingers',
  'Acrobat', 'Healer', 'Navigator', 'Sharpshooter', 'Swashbuckler',
  'Animal Whisperer', 'Deadly Aim', 'Light Feet', 'Tracker',
  'Hold the Line', 'I Won’t Let You Down', 'Wolf in Sheep’s Clothing',
];

export const CONNECTION_PROMPTS = [
  'Why do you trust me so deeply?',
  'What did I do that makes you cautious around me?',
  'Why do we keep our shared past a secret?',
];

export interface WeaponDef {
  id: string;
  name: string;
  trait: TraitId;
  range: string;
  damage: string;
  hands: 1 | 2;
}

export interface ArmorDef {
  id: string;
  name: string;
  major: number;
  severe: number;
  score: number;
  evasion: number;
  note: string;
}

export interface DomainCardDef {
  id: string;
  domain: string;
  name: string;
  level: number;
  kind: string;
  recall: number;
  text: string;
}

export interface SubclassDef {
  id: string;
  name: string;
  pitch: string;
  spellcast: string;
  foundation: string;
  specialization: string;
  mastery: string;
}

export interface ClassDef {
  id: string;
  name: string;
  pitch: string;
  description: string;
  domains: [string, string];
  evasion: number;
  hp: number;
  spellcast: TraitId;
  spellcastLabel: string;
  features: string[];
  hope: string;
  suggested: Record<TraitId, number>;
  weapon: string;
  armor: string;
  keepsakes: [string, string];
  questions: [string, string, string];
  connections: [string, string, string];
  subclasses: SubclassDef[];
}

export const WEAPONS: WeaponDef[] = [
  { id: 'battleaxe', name: 'Battleaxe', trait: 'strength', range: 'Melee', damage: '1d10+1 phy', hands: 2 },
  { id: 'broadsword', name: 'Broadsword', trait: 'strength', range: 'Melee', damage: '1d8 phy', hands: 1 },
  { id: 'crossbow', name: 'Crossbow', trait: 'finesse', range: 'Far', damage: '1d6+1 phy', hands: 1 },
  { id: 'dualstaff', name: 'Dualstaff', trait: 'instinct', range: 'Far', damage: '1d6+3 mag', hands: 2 },
  { id: 'greatsword', name: 'Greatsword', trait: 'strength', range: 'Melee', damage: '1d10+3 phy', hands: 2 },
  { id: 'longbow', name: 'Longbow', trait: 'agility', range: 'Far', damage: '1d8+3 phy', hands: 2 },
  { id: 'mace', name: 'Mace', trait: 'strength', range: 'Melee', damage: '1d8+1 phy', hands: 1 },
  { id: 'quarterstaff', name: 'Quarterstaff', trait: 'instinct', range: 'Melee', damage: '1d10+1 phy', hands: 2 },
  { id: 'rapier', name: 'Rapier', trait: 'presence', range: 'Melee', damage: '1d8 phy', hands: 1 },
  { id: 'shortbow', name: 'Shortbow', trait: 'agility', range: 'Far', damage: '1d6+3 phy', hands: 2 },
  { id: 'shortsword', name: 'Shortsword', trait: 'finesse', range: 'Melee', damage: '1d8 phy', hands: 1 },
  { id: 'warhammer', name: 'Warhammer', trait: 'strength', range: 'Melee', damage: '1d12+3 phy', hands: 2 },
];

export const OFFHAND: WeaponDef[] = [
  { id: 'dagger', name: 'Dagger', trait: 'finesse', range: 'Melee', damage: '1d8 phy', hands: 1 },
  { id: 'handaxe', name: 'Handaxe', trait: 'strength', range: 'Melee', damage: '1d6 phy', hands: 1 },
  { id: 'whip', name: 'Whip', trait: 'presence', range: 'Very Close', damage: '1d6 phy', hands: 1 },
];

export const ARMORS: ArmorDef[] = [
  { id: 'gambeson', name: 'Gambeson', major: 5, severe: 11, score: 3, evasion: 1, note: 'Flexible. +1 Evasion.' },
  { id: 'leather', name: 'Leather', major: 6, severe: 13, score: 3, evasion: 0, note: 'A light coat of hardened hide.' },
  { id: 'chainmail', name: 'Chainmail', major: 7, severe: 15, score: 4, evasion: -1, note: 'Heavy. −1 Evasion.' },
  { id: 'plate', name: 'Full Plate', major: 8, severe: 17, score: 4, evasion: -2, note: 'Very heavy. −2 Evasion.' },
];

export interface AdversaryTemplate {
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
}

export function classById(id: string): ClassDef | undefined {
  return CLASSES.find((item) => item.id === id);
}

export function weaponById(id: string): WeaponDef | undefined {
  return WEAPONS.find((item) => item.id === id) ?? OFFHAND.find((item) => item.id === id);
}

export function armorById(id: string): ArmorDef | undefined {
  return ARMORS.find((item) => item.id === id);
}

export function cardsFor(classId: string): DomainCardDef[] {
  const chosen = classById(classId);
  if (!chosen) return [];
  return DOMAIN_CARDS.filter((card) => card.level === 1 && chosen.domains.includes(card.domain));
}

export function diceOf(damage: string): string {
  let text = damage.trim();
  const lower = text.toLowerCase();
  for (const suffix of [' direct phy', ' direct mag', ' phy', ' mag']) {
    if (lower.endsWith(suffix)) {
      text = text.slice(0, -suffix.length).trim();
      break;
    }
  }
  return /^\d+$/.test(text) ? `flat:${text}` : text;
}
