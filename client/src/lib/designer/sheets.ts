// ------------------------------------------------------------
// Module: sheets
// Purpose: Plain helpers for naming a thing and placing it
//          on a PC, NPC, or adversary sheet.
// ------------------------------------------------------------

import type { CharacterKind } from '../net/bindings/CharacterKind.ts';
import type { CharacterSheets } from '../net/bindings/CharacterSheets.ts';
import type { FieldDef } from '../net/bindings/FieldDef.ts';
import type { FieldKind } from '../net/bindings/FieldKind.ts';
import type { KindSheet } from '../net/bindings/KindSheet.ts';
import type { SystemDef } from '../net/bindings/SystemDef.ts';

export const SHEET_KINDS: { kind: CharacterKind; title: string }[] = [
  { kind: 'pc', title: 'PC' },
  { kind: 'npc', title: 'NPC' },
  { kind: 'adversary', title: 'Adversary' },
];

export const THING_KINDS: { kind: FieldKind; title: string; line: string }[] = [
  { kind: 'number', title: 'Number', line: 'A single value, such as Evasion or Level.' },
  { kind: 'text', title: 'Text', line: 'Words, such as Ancestry or Notes.' },
  { kind: 'choice', title: 'Choice', line: 'One option from a list, such as an adversary Type.' },
  { kind: 'pool', title: 'Pool', line: 'A value you mark up to a maximum, such as HP or Stress.' },
  { kind: 'thresholds', title: 'Thresholds', line: 'Named bands, such as Minor and Major. Apply damage marks a pool.' },
  { kind: 'dice', title: 'Dice', line: 'A roll formula, such as weapon damage.' },
];

export function emptySheet(): KindSheet {
  return { sections: [], toolbar: { resources: [], actions: [] } };
}

export function emptySheets(): CharacterSheets {
  return { pc: emptySheet(), npc: emptySheet(), adversary: emptySheet() };
}

export function thingId(name: string, taken: string[]): string {
  return uniqueId(name, taken, 24, 'thing');
}

export function systemId(name: string, taken: string[]): string {
  const id = uniqueId(name, taken, 32, 'system');
  return id.length >= 2 ? id : uniqueId('system', taken, 32, 'system');
}

export function placeOn(sheets: CharacterSheets, kind: CharacterKind, fieldId: string): CharacterSheets {
  const next = structuredClone(sheets);
  const sheet = next[kind];
  if (sheet.sections.some((section) => section.fields.includes(fieldId))) return next;
  const last = sheet.sections[sheet.sections.length - 1];
  if (!last) sheet.sections.push({ title: 'Sheet', columns: 2, fields: [fieldId] });
  else last.fields.push(fieldId);
  return next;
}

export function removeFrom(sheets: CharacterSheets, kind: CharacterKind, fieldId: string): CharacterSheets {
  const next = detach(sheets, kind, fieldId);
  const sheet = next[kind];
  sheet.toolbar.resources = sheet.toolbar.resources.filter((id) => id !== fieldId);
  sheet.toolbar.actions = sheet.toolbar.actions.filter((id) => id !== fieldId);
  return next;
}

export function detach(sheets: CharacterSheets, kind: CharacterKind, fieldId: string): CharacterSheets {
  const next = structuredClone(sheets);
  for (const section of next[kind].sections) {
    section.fields = section.fields.filter((id) => id !== fieldId);
  }
  return next;
}

export function insertIn(sheets: CharacterSheets, kind: CharacterKind, fieldId: string, sectionIndex: number, beforeId: string | null): CharacterSheets {
  const next = detach(sheets, kind, fieldId);
  const section = next[kind].sections[sectionIndex];
  if (!section) return next;
  if (!beforeId) section.fields.push(fieldId);
  else {
    const at = section.fields.indexOf(beforeId);
    if (at < 0) section.fields.push(fieldId);
    else section.fields.splice(at, 0, fieldId);
  }
  return next;
}

export function onSheet(sheets: CharacterSheets, kind: CharacterKind, fieldId: string): boolean {
  return sheets[kind].sections.some((section) => section.fields.includes(fieldId));
}

export function takenIds(system: SystemDef): string[] {
  const ids = system.fields.flatMap((field) => [field.id, ...field.steps.map((step) => step.id)]);
  return ids.concat(system.actions.map((action) => action.id));
}

export function blankThing(kind: FieldKind, name: string, taken: string[]): FieldDef {
  const id = thingId(name, taken);
  const stepTaken = taken.concat(id);
  return {
    id,
    name,
    kind,
    min: 0,
    max: kind === 'pool' ? 6 : null,
    maxFrom: null,
    default: 0,
    textDefault: '',
    look: kind === 'pool' ? 'boxes' : 'boxes',
    options: kind === 'choice' ? ['First'] : [],
    steps: kind === 'thresholds' ? [{ id: thingId('Minor', stepTaken), name: 'Minor', default: 1, marks: 1 }] : [],
    pool: null,
    formula: kind === 'dice' ? '1d20' : '',
    hint: '',
  };
}

export function kindTitle(kind: FieldKind): string {
  return THING_KINDS.find((item) => item.kind === kind)?.title ?? kind;
}

function uniqueId(name: string, taken: string[], limit: number, fallback: string): string {
  const slug = name.toLowerCase().replace(/[^a-z0-9]/g, '');
  let stem = slug.replace(/^[0-9]+/, '');
  if (!/^[a-z]/.test(stem)) stem = fallback;
  stem = stem.slice(0, limit);
  const used = new Set(taken);
  let id = stem;
  let count = 2;
  while (used.has(id) || id.length === 0) {
    const suffix = String(count);
    id = `${stem.slice(0, limit - suffix.length)}${suffix}`;
    count += 1;
  }
  return id;
}
