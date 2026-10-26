// ------------------------------------------------------------
// Module: describe
// Purpose: Turn a thing or a formula into one sentence a GM
//          can read without opening its settings.
// ------------------------------------------------------------

import type { FieldDef } from '../net/bindings/FieldDef.ts';
import type { SystemDef } from '../net/bindings/SystemDef.ts';

export function describeThing(field: FieldDef, system: SystemDef): string {
  if (field.kind === 'number') {
    const ceiling = field.max === null ? 'no maximum' : String(field.max);
    return `A number from ${field.min} to ${ceiling}, starting at ${field.default}.`;
  }
  if (field.kind === 'text') return 'Words.';
  if (field.kind === 'choice') {
    const options = field.options.map((option) => option.trim()).filter(Boolean);
    if (options.length === 0) return 'A choice with no options yet.';
    if (options.length === 1) return `One of ${options[0]}.`;
    const last = options[options.length - 1];
    return `One of ${options.slice(0, -1).join(', ')}, or ${last}.`;
  }
  if (field.kind === 'pool') {
    const look = field.look === 'count' ? 'a count' : field.look;
    if (field.maxFrom) {
      const source = system.fields.find((item) => item.id === field.maxFrom);
      const name = source?.name ?? field.maxFrom;
      const shown = source ? ` (${source.default})` : '';
      return `A pool of ${look}; its maximum comes from ${name}${shown}.`;
    }
    const cap = field.max === null ? 'no fixed maximum' : String(field.max);
    return `A pool of ${look}, up to ${cap}.`;
  }
  if (field.kind === 'thresholds') {
    if (field.steps.length === 0) return 'Thresholds with no steps yet.';
    const pool = system.fields.find((item) => item.id === field.pool);
    const unit = pool?.name ?? '';
    const steps = field.steps.map((step) => {
      const marks = unit ? `${step.marks} ${unit}` : String(step.marks);
      return `${step.name} at ${step.default} marks ${marks}`;
    });
    return `${steps.join(', ')}.`;
  }
  if (!field.formula.trim()) return 'A roll with no formula yet.';
  return `A roll: ${describeFormula(field.formula, system)}.`;
}

export function describeFormula(formula: string, system: SystemDef): string {
  const tags = new Map(system.dieTags.map((tag) => [tag.letter, tag.name]));
  const names = new Map<string, string>();
  for (const field of system.fields) {
    names.set(field.id, field.name);
    for (const step of field.steps) names.set(step.id, step.name);
  }
  const named = formula.replace(/(\d*)d(\d+)([a-z])?/gi, (_match, count: string, sides: string, letter?: string) => {
    const amount = count === '' || count === '1' ? '' : `${count} `;
    const tag = letter ? tags.get(letter.toLowerCase()) : undefined;
    return tag ? `${amount}${tag} d${sides}` : `${amount}d${sides}`;
  });
  const filled = named.replace(/@([a-zA-Z0-9_]+)/g, (_match, id: string) => names.get(id) ?? id);
  return filled.replace(/\s*\+\s*/g, ' + ').replace(/\s*-\s*/g, ' − ').replace(/\s+/g, ' ').trim();
}
