// ------------------------------------------------------------
// Module: catalog
// Purpose: The choices at a Daggerheart table while the class
//          features and domain cards are still basic. Names and
//          starting numbers live here. The rules text comes later.
// ------------------------------------------------------------

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

export const ANCESTRIES = [
  'Clank', 'Drakona', 'Dwarf', 'Elf', 'Faerie', 'Faun', 'Firbolg', 'Fungril',
  'Galapa', 'Giant', 'Goblin', 'Halfling', 'Human', 'Infernis', 'Katari', 'Orc', 'Ribbet', 'Simiah',
];

export const COMMUNITIES = [
  'Highborne', 'Loreborne', 'Orderborne', 'Ridgeborne', 'Seaborne', 'Slyborne', 'Underborne', 'Wanderborne', 'Wildborne',
];

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
}

export interface ClassDef {
  id: string;
  name: string;
  pitch: string;
  domains: [string, string];
  evasion: number;
  hp: number;
  spellcast: TraitId;
  features: string[];
  suggested: Record<TraitId, number>;
  weapon: string;
  armor: string;
  keepsakes: [string, string];
  questions: [string, string, string];
  subclasses: { id: string; name: string; pitch: string }[];
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

const DOMAINS = ['Arcana', 'Blade', 'Bone', 'Codex', 'Grace', 'Midnight', 'Sage', 'Splendor', 'Valor'];

export const DOMAIN_CARDS: DomainCardDef[] = DOMAINS.flatMap((domain) => [
  { id: `${domain.toLowerCase()}-1`, domain, name: `${domain} · first card` },
  { id: `${domain.toLowerCase()}-2`, domain, name: `${domain} · second card` },
]);

function traits(agility: number, strength: number, finesse: number, instinct: number, presence: number, knowledge: number): Record<TraitId, number> {
  return { agility, strength, finesse, instinct, presence, knowledge };
}

export const CLASSES: ClassDef[] = [
  {
    id: 'bard',
    name: 'Bard',
    pitch: 'The voice of the party. Music, stories, and a presence that turns a room.',
    domains: ['Grace', 'Codex'],
    evasion: 10,
    hp: 5,
    spellcast: 'presence',
    features: ['Rally'],
    suggested: traits(1, -1, 1, 0, 2, 0),
    weapon: 'rapier',
    armor: 'gambeson',
    keepsakes: ['a well-worn songbook', 'a letter of introduction'],
    questions: [
      'What song or story do people already know you for?',
      'Who did you leave when you took to the road, and what do you still owe them?',
      'What do you hope this party will remember you for?',
    ],
    subclasses: [
      { id: 'troubadour', name: 'Troubadour', pitch: 'Play music to bolster your allies.' },
      { id: 'wordsmith', name: 'Wordsmith', pitch: 'Use clever wordplay and captivate crowds.' },
    ],
  },
  {
    id: 'druid',
    name: 'Druid',
    pitch: 'The wild, embodied. Elements, beasts, and the magic that mends.',
    domains: ['Sage', 'Arcana'],
    evasion: 10,
    hp: 6,
    spellcast: 'instinct',
    features: ['Beastform'],
    suggested: traits(0, 1, -1, 2, 0, 1),
    weapon: 'quarterstaff',
    armor: 'leather',
    keepsakes: ['a bundle of herbs', 'a carved beast token'],
    questions: [
      'What part of the wild do you answer to?',
      'What did you fail to protect, and what grew back in its place?',
      'What do the people of your community ask of you when the weather turns?',
    ],
    subclasses: [
      { id: 'elements', name: 'Warden of the Elements', pitch: 'Embody the natural elements of the wild.' },
      { id: 'renewal', name: 'Warden of Renewal', pitch: 'Use powerful magic to heal your party.' },
    ],
  },
  {
    id: 'guardian',
    name: 'Guardian',
    pitch: 'The one who steps in front. You take the blow so someone else does not.',
    domains: ['Valor', 'Blade'],
    evasion: 9,
    hp: 7,
    spellcast: 'strength',
    features: ['Unstoppable'],
    suggested: traits(-1, 2, 0, 1, 1, 0),
    weapon: 'battleaxe',
    armor: 'chainmail',
    keepsakes: ['a family shield charm', 'a medal you do not talk about'],
    questions: [
      'Who did you stand in front of, and are they still alive?',
      'What oath did you take, and who holds you to it?',
      'What are you afraid will happen if you are not the first one hit?',
    ],
    subclasses: [
      { id: 'stalwart', name: 'Stalwart', pitch: 'Take heavy blows and keep fighting.' },
      { id: 'vengeance', name: 'Vengeance', pitch: 'Strike down enemies who harm you or your allies.' },
    ],
  },
  {
    id: 'ranger',
    name: 'Ranger',
    pitch: 'The hunt. An ally at your side, or a shot that ends the chase.',
    domains: ['Bone', 'Sage'],
    evasion: 11,
    hp: 6,
    spellcast: 'agility',
    features: ["Ranger's Focus"],
    suggested: traits(2, 0, 1, 1, -1, 0),
    weapon: 'shortbow',
    armor: 'leather',
    keepsakes: ['a hunting trophy', 'a map of a place you have not been'],
    questions: [
      'What are you hunting, and how long has the trail been cold?',
      'Who taught you to read a wood, and what did they refuse to teach you?',
      'What do you do when the quarry turns out to be a person?',
    ],
    subclasses: [
      { id: 'beastbound', name: 'Beastbound', pitch: 'Form a deep bond with an animal ally.' },
      { id: 'wayfinder', name: 'Wayfinder', pitch: 'Hunt your prey and strike with deadly force.' },
    ],
  },
  {
    id: 'rogue',
    name: 'Rogue',
    pitch: 'The shadow and the favor. You get in, and you know someone who can.',
    domains: ['Midnight', 'Grace'],
    evasion: 12,
    hp: 6,
    spellcast: 'finesse',
    features: ['Cloaked'],
    suggested: traits(1, -1, 2, 1, 0, 0),
    weapon: 'shortsword',
    armor: 'gambeson',
    keepsakes: ['a forged seal', "a contact's token"],
    questions: [
      'What did you steal that you still have not sold?',
      'Who in your community would open a door for you, and who would close one?',
      'What name do you use when your own is a problem?',
    ],
    subclasses: [
      { id: 'nightwalker', name: 'Nightwalker', pitch: 'Manipulate shadows to maneuver through the environment.' },
      { id: 'syndicate', name: 'Syndicate', pitch: 'Keep a web of contacts everywhere you go.' },
    ],
  },
  {
    id: 'seraph',
    name: 'Seraph',
    pitch: 'The oath. A legendary weapon, or wings over the fight.',
    domains: ['Splendor', 'Valor'],
    evasion: 9,
    hp: 7,
    spellcast: 'strength',
    features: ['Prayer Dice'],
    suggested: traits(0, 2, -1, 1, 1, 0),
    weapon: 'warhammer',
    armor: 'chainmail',
    keepsakes: ['a prayer bead', 'a relic splinter'],
    questions: [
      'What did you swear, and who heard you?',
      'What would make you break that oath?',
      'Who looks at you and sees the office, not the person?',
    ],
    subclasses: [
      { id: 'winged', name: 'Winged Sentinel', pitch: 'Take flight and strike crushing blows from the sky.' },
      { id: 'divine', name: 'Divine Wielder', pitch: 'Dominate the battlefield with a legendary weapon.' },
    ],
  },
  {
    id: 'sorcerer',
    name: 'Sorcerer',
    pitch: 'Magic you were born with. An element, or a spell bent further than it was written.',
    domains: ['Arcana', 'Midnight'],
    evasion: 10,
    hp: 6,
    spellcast: 'instinct',
    features: ['Arcane Sense', 'Minor Illusion', 'Channel Raw Power'],
    suggested: traits(0, -1, 1, 2, 1, 0),
    weapon: 'dualstaff',
    armor: 'gambeson',
    keepsakes: ['a whispering orb', 'a family heirloom'],
    questions: [
      'What did you do that made the people in your community wary of you?',
      'Who taught you to hold your magic, and why are they gone?',
      'What fear do you hide, and why does it scare you?',
    ],
    subclasses: [
      { id: 'elemental', name: 'Elemental Origin', pitch: 'Channel the raw power of a particular element.' },
      { id: 'primal', name: 'Primal Origin', pitch: 'Extend the versatility of your spells in powerful ways.' },
    ],
  },
  {
    id: 'warrior',
    name: 'Warrior',
    pitch: 'The front line. Their strength becomes yours, or you simply hit harder.',
    domains: ['Blade', 'Bone'],
    evasion: 11,
    hp: 6,
    spellcast: 'strength',
    features: ['Attack of Opportunity'],
    suggested: traits(1, 2, 1, 0, 0, -1),
    weapon: 'greatsword',
    armor: 'leather',
    keepsakes: ['a notched fight token', 'a scrap of a banner'],
    questions: [
      'What battle are you still fighting in your head?',
      'Who did you refuse to leave, and what did it cost?',
      'What weapon feels like yours, and where did you get it?',
    ],
    subclasses: [
      { id: 'brave', name: 'Call of the Brave', pitch: 'Use the might of your enemies to fuel your own power.' },
      { id: 'slayer', name: 'Call of the Slayer', pitch: 'Strike down adversaries with immense force.' },
    ],
  },
  {
    id: 'wizard',
    name: 'Wizard',
    pitch: 'The studied spell. You understand the working, or you were trained to make it violent.',
    domains: ['Codex', 'Splendor'],
    evasion: 11,
    hp: 5,
    spellcast: 'knowledge',
    features: ['Prestidigitation', 'Strange Patterns'],
    suggested: traits(0, -1, 0, 1, 1, 2),
    weapon: 'quarterstaff',
    armor: 'gambeson',
    keepsakes: ['a spare spellbook', "a mentor's bookmark"],
    questions: [
      'What have you read that you were not supposed to?',
      'Who paid for your study, and what do they expect back?',
      'What question are you still trying to answer?',
    ],
    subclasses: [
      { id: 'knowledge', name: 'School of Knowledge', pitch: 'Keep a keen understanding of the world around you.' },
      { id: 'war', name: 'School of War', pitch: 'Use trained magic for violence.' },
    ],
  },
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

export const JAGGED_KNIFE_BANDIT: AdversaryTemplate = {
  name: 'Jagged Knife Bandit',
  type: 'Standard',
  tier: 1,
  description: "A cunning criminal in a cloak bearing one of the gang's iconic knives.",
  motives: 'Escape, profit, steal, throw smoke',
  difficulty: 12,
  major: 8,
  severe: 14,
  hp: 5,
  stress: 3,
  attack: 1,
  attackName: 'Daggers',
  attackRange: 'Melee',
  damage: '1d8+1',
  experience: 'Thief +2',
  features: [
    'Climber — Passive: The Bandit climbs just as easily as they run.',
    'From Above — Passive: When the Bandit succeeds on a standard attack from above a target, they deal 1d10+1 physical damage instead of their standard damage.',
  ].join('\n'),
};

export const ADVERSARY_TYPES = ['Standard', 'Minion', 'Horde', 'Ranged', 'Skulk', 'Bruiser', 'Leader', 'Solo', 'Social', 'Support'];

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
  return DOMAIN_CARDS.filter((card) => chosen.domains.includes(card.domain));
}

export function diceOf(damage: string): string {
  return damage.replace(/\s*(phy|mag)$/i, '').trim();
}
