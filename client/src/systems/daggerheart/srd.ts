// ------------------------------------------------------------
// Module: srd
// Purpose: The Daggerheart SRD, turned into the lists the table
//          uses. Classes, domain cards, adversaries, ancestries,
//          communities, and the rule chapters.
// ------------------------------------------------------------

import abilitiesJson from '../../../../docs/Systems/daggerheart-srd-2.0-main/.build/03_json/abilities.json';
import adversariesJson from '../../../../docs/Systems/daggerheart-srd-2.0-main/.build/03_json/adversaries.json';
import ancestriesJson from '../../../../docs/Systems/daggerheart-srd-2.0-main/.build/03_json/ancestries.json';
import classesJson from '../../../../docs/Systems/daggerheart-srd-2.0-main/.build/03_json/classes.json';
import communitiesJson from '../../../../docs/Systems/daggerheart-srd-2.0-main/.build/03_json/communities.json';
import rulesJson from '../../../../docs/Systems/daggerheart-srd-2.0-main/.build/03_json/core_rules.json';
import subclassesJson from '../../../../docs/Systems/daggerheart-srd-2.0-main/.build/03_json/subclasses.json';
import type { AdversaryTemplate, ClassDef, DomainCardDef, SubclassDef, TraitId } from './catalog';

interface NamedText {
  name: string;
  text: string;
}

interface SrdQuestion {
  question: string;
}

interface SrdClass {
  name: string;
  description: string;
  domain_1: string;
  domain_2: string;
  evasion: string;
  hp: string;
  items: string;
  hope_feature_name: string;
  hope_feature_text: string;
  feature: NamedText[];
  background: SrdQuestion[];
  connection: SrdQuestion[];
  subclass_1: string;
  subclass_2: string;
}

interface SrdSubclass {
  name: string;
  description: string;
  spellcast_trait?: string;
  foundation: NamedText[];
  specialization: NamedText[];
  mastery: NamedText[];
}

interface SrdCard {
  domain: string;
  level: string;
  name: string;
  recall: string;
  text: string;
  type: string;
}

interface SrdAdversary {
  name: string;
  type: string;
  tier: string;
  description: string;
  motives_and_tactics: string;
  difficulty: string;
  thresholds: string;
  hp: string;
  stress: string;
  atk: string;
  attack: string;
  range: string;
  damage: string;
  experience: string;
  feature: NamedText[];
}

interface SrdHeritage {
  name: string;
  description: string;
  feature: NamedText[];
  note?: string;
}

interface SrdRule {
  name: string;
  text: string;
}

export interface HeritageDef {
  name: string;
  description: string;
  feature: string;
}

export interface RuleSection {
  name: string;
  text: string;
}

const TRAIT_IDS: TraitId[] = ['agility', 'strength', 'finesse', 'instinct', 'presence', 'knowledge'];

const SUBCLASS_IDS: Record<string, string> = {
  'Warden of the Elements': 'elements',
  'Warden of Renewal': 'renewal',
  'Winged Sentinel': 'winged',
  'Divine Wielder': 'divine',
  'Elemental Origin': 'elemental',
  'Primal Origin': 'primal',
  'Call of the Brave': 'brave',
  'Call of the Slayer': 'slayer',
  'School of Knowledge': 'knowledge',
  'School of War': 'war',
};

const GEAR: Record<string, { weapon: string; armor: string }> = {
  Bard: { weapon: 'rapier', armor: 'gambeson' },
  Druid: { weapon: 'quarterstaff', armor: 'leather' },
  Guardian: { weapon: 'battleaxe', armor: 'chainmail' },
  Ranger: { weapon: 'shortbow', armor: 'leather' },
  Rogue: { weapon: 'shortsword', armor: 'gambeson' },
  Seraph: { weapon: 'warhammer', armor: 'chainmail' },
  Sorcerer: { weapon: 'dualstaff', armor: 'gambeson' },
  Warrior: { weapon: 'greatsword', armor: 'leather' },
  Wizard: { weapon: 'quarterstaff', armor: 'gambeson' },
};

const GEAR_FOR: Record<TraitId, { weapon: string; armor: string }> = {
  agility: { weapon: 'shortbow', armor: 'leather' },
  strength: { weapon: 'battleaxe', armor: 'chainmail' },
  finesse: { weapon: 'shortsword', armor: 'gambeson' },
  instinct: { weapon: 'quarterstaff', armor: 'leather' },
  presence: { weapon: 'rapier', armor: 'gambeson' },
  knowledge: { weapon: 'quarterstaff', armor: 'gambeson' },
};

const SUGGESTED: Record<string, Record<TraitId, number>> = {
  Bard: traits(1, -1, 1, 0, 2, 0),
  Druid: traits(0, 1, -1, 2, 0, 1),
  Guardian: traits(-1, 2, 0, 1, 1, 0),
  Ranger: traits(2, 0, 1, 1, -1, 0),
  Rogue: traits(1, -1, 2, 1, 0, 0),
  Seraph: traits(0, 2, -1, 1, 1, 0),
  Sorcerer: traits(0, -1, 1, 2, 1, 0),
  Warrior: traits(1, 2, 1, 0, 0, -1),
  Wizard: traits(0, -1, 0, 1, 1, 2),
};

function traits(agility: number, strength: number, finesse: number, instinct: number, presence: number, knowledge: number): Record<TraitId, number> {
  return { agility, strength, finesse, instinct, presence, knowledge };
}

export function plain(text: string): string {
  return text.replace(/\*\*(.+?)\*\*/g, '$1').replace(/_(.+?)_/g, '$1').trim();
}

export function ruleHtml(source: string): string {
  const escaped = source.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  const marked = escaped
    .replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>');
  return marked.split(/\n{2,}/).map((block) => {
    const lines = block.split('\n').map((line) => line.trim()).filter((line) => line.length > 0);
    const heading = lines[0] ?? '';
    if (isRuleTable(lines)) return tableHtml(lines);
    if (heading.startsWith('#### ')) return `<h3>${heading.slice(5)}</h3>${body(lines.slice(1))}`;
    if (heading.startsWith('### ')) return `<h3>${heading.slice(4)}</h3>${body(lines.slice(1))}`;
    if (heading.startsWith('## ')) return `<h2>${heading.slice(3)}</h2>${body(lines.slice(1))}`;
    return `<p>${lines.map((line) => (line.startsWith('- ') ? `• ${line.slice(2)}` : line)).join('<br>')}</p>`;
  }).join('');
}

function isRuleTable(lines: string[]): boolean {
  return lines.length >= 2 && lines.every((line) => line.startsWith('|') && line.endsWith('|')) && /^\|(?:\s*:?-{3,}:?\s*\|)+$/.test(lines[1] ?? '');
}

function tableCells(line: string): string[] {
  return line.replace(/^\|/, '').replace(/\|$/, '').split('|').map((cell) => cell.trim());
}

function tableHtml(lines: string[]): string {
  const header = tableCells(lines[0] ?? '').map((cell) => `<th>${cell}</th>`).join('');
  const rows = lines.slice(2).map((line) => `<tr>${tableCells(line).map((cell) => `<td>${cell}</td>`).join('')}</tr>`).join('');
  return `<div class="rule-table"><table><thead><tr>${header}</tr></thead><tbody>${rows}</tbody></table></div>`;
}

function body(lines: string[]): string {
  const text = lines.join('<br>').trim();
  return text ? `<p>${text}</p>` : '';
}

function slug(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

function sentence(text: string): string {
  const clean = plain(text).replace(/\s+/g, ' ').trim();
  const end = clean.search(/[.!?](\s|$)/);
  const line = end >= 0 ? clean.slice(0, end + 1) : clean;
  return line.length > 220 ? `${line.slice(0, 217)}…` : line;
}

function block(feature: NamedText): string {
  return `${plain(feature.name)}\n${plain(feature.text)}`;
}

function blocks(features: NamedText[]): string {
  return features.map(block).join('\n\n');
}

function three(items: SrdQuestion[] | undefined): [string, string, string] {
  const list = (items ?? []).map((item) => item.question);
  while (list.length < 3) list.push('What else should the table know?');
  return [list[0] ?? '', list[1] ?? '', list[2] ?? ''];
}

function traitId(name: string | undefined): TraitId | null {
  const id = name?.toLowerCase();
  return TRAIT_IDS.find((trait) => trait === id) ?? null;
}

function suggest(primary: TraitId): Record<TraitId, number> {
  const rest = TRAIT_IDS.filter((id) => id !== primary);
  const order = [primary, ...rest];
  const values = [2, 1, 1, 0, 0, -1];
  const result = traits(0, 0, 0, 0, 0, 0);
  order.forEach((id, index) => {
    result[id] = values[index] ?? 0;
  });
  return result;
}

function keepsakes(items: string): [string, string] {
  const parts = items.split(/\s+or\s+/i).map((part) => part.trim()).filter(Boolean);
  if (parts.length >= 2) return [parts[0] ?? items, parts[1] ?? 'a personal token'];
  return [items.trim() || 'a personal token', 'a token from home'];
}

function integer(value: string, fallback: number): number {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
}

const subclasses = new Map((subclassesJson as SrdSubclass[]).map((item) => [item.name, item]));

function subclassOf(name: string): SubclassDef {
  const source = subclasses.get(name);
  const spell = source?.spellcast_trait ?? '';
  return {
    id: SUBCLASS_IDS[name] ?? slug(name),
    name,
    pitch: source ? sentence(source.description) : name,
    spellcast: spell,
    foundation: source ? blocks(source.foundation) : '',
    specialization: source ? blocks(source.specialization) : '',
    mastery: source ? blocks(source.mastery) : '',
  };
}

export const CLASSES: ClassDef[] = (classesJson as SrdClass[]).map((item) => {
  const first = subclassOf(item.subclass_1);
  const second = subclassOf(item.subclass_2);
  const spells = [traitId(first.spellcast), traitId(second.spellcast)].filter((trait): trait is TraitId => trait !== null);
  const unique = [...new Set(spells)];
  const spellcast = unique[0] ?? 'strength';
  const gear = GEAR[item.name] ?? GEAR_FOR[spellcast];
  return {
    id: slug(item.name),
    name: item.name,
    pitch: sentence(item.description),
    description: plain(item.description),
    domains: [item.domain_1, item.domain_2],
    evasion: integer(item.evasion, 10),
    hp: integer(item.hp, 6),
    spellcast,
    spellcastLabel: unique.length > 0 ? unique.map((id) => id[0]?.toUpperCase() + id.slice(1)).join(' or ') : '—',
    features: item.feature.map(block),
    hope: block({ name: item.hope_feature_name, text: item.hope_feature_text }),
    suggested: SUGGESTED[item.name] ?? suggest(spellcast),
    weapon: gear.weapon,
    armor: gear.armor,
    keepsakes: keepsakes(item.items),
    questions: three(item.background),
    connections: three(item.connection),
    subclasses: [first, second],
  };
});

export const DOMAIN_CARDS: DomainCardDef[] = (abilitiesJson as SrdCard[]).map((card) => ({
  id: `${slug(card.domain)}-${slug(card.name)}`,
  domain: card.domain,
  name: card.name,
  level: integer(card.level, 1),
  kind: card.type,
  recall: integer(card.recall, 0),
  text: plain(card.text),
}));

function thresholds(raw: string): { major: number; severe: number } {
  if (!raw || raw === 'None') return { major: 99, severe: 99 };
  const [major, severe] = raw.split('/');
  return {
    major: !major || major === 'None' ? 99 : integer(major, 99),
    severe: !severe || severe === 'None' ? 99 : integer(severe, 99),
  };
}

function adversaryOf(item: SrdAdversary): AdversaryTemplate {
  const gates = thresholds(item.thresholds);
  const flat = /^[+-]\d+$/.test(item.atk);
  const bonus = flat ? '' : `Standard attack adds ${item.atk} instead of a flat bonus.`;
  const features = [bonus, ...item.feature.map((feature) => `${plain(feature.name)} — ${plain(feature.text)}`)].filter(Boolean).join('\n');
  return {
    name: item.name,
    type: item.type,
    tier: integer(item.tier, 1),
    description: plain(item.description),
    motives: item.motives_and_tactics,
    difficulty: integer(item.difficulty, 11),
    major: gates.major,
    severe: gates.severe,
    hp: integer(item.hp, 1),
    stress: item.stress === 'None' ? 0 : integer(item.stress, 0),
    attack: flat ? integer(item.atk, 0) : 0,
    attackName: flat ? item.attack : `${item.attack} (${item.atk})`,
    attackRange: item.range,
    damage: item.damage,
    experience: item.experience === 'None' ? '' : item.experience,
    features,
  };
}

export const ADVERSARIES: AdversaryTemplate[] = (adversariesJson as SrdAdversary[]).map(adversaryOf);

export const ADVERSARY_TYPES = [...new Set(ADVERSARIES.map((item) => item.type))].sort((a, b) => a.localeCompare(b));

export const JAGGED_KNIFE_BANDIT = ADVERSARIES.find((item) => item.name === 'Jagged Knife Bandit')
  ?? ADVERSARIES[0]
  ?? adversaryOf({
    name: 'Adversary', type: 'Standard', tier: '1', description: '', motives_and_tactics: '',
    difficulty: '11', thresholds: '8/14', hp: '5', stress: '3', atk: '+1', attack: 'Attack',
    range: 'Melee', damage: '1d8', experience: '', feature: [],
  });

function heritage(item: SrdHeritage): HeritageDef {
  const note = item.note ? `\n${plain(item.note)}` : '';
  return {
    name: item.name,
    description: sentence(item.description),
    feature: `${blocks(item.feature)}${note}`,
  };
}

export const ANCESTRIES: HeritageDef[] = (ancestriesJson as SrdHeritage[]).map(heritage);
export const COMMUNITIES: HeritageDef[] = (communitiesJson as SrdHeritage[]).map(heritage);
export const RULES: RuleSection[] = (rulesJson as SrdRule[]).map((rule) => ({ name: rule.name, text: rule.text }));

export function ancestryByName(name: string): HeritageDef | undefined {
  return ANCESTRIES.find((item) => item.name === name);
}

export function communityByName(name: string): HeritageDef | undefined {
  return COMMUNITIES.find((item) => item.name === name);
}
