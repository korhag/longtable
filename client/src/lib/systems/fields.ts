// ------------------------------------------------------------
// Module: fields
// Purpose: Shared reading of a thing: its ceiling, the step
//          damage marks, and a preview of a filled formula.
// ------------------------------------------------------------

import type { FieldDef } from '../net/bindings/FieldDef.ts';

export function ceiling(field: FieldDef, numbers: Record<string, number>): number {
  if (field.maxFrom) {
    const from = numbers[field.maxFrom];
    if (from !== undefined) return Math.max(0, from);
  }
  return Math.max(0, field.max ?? 1000);
}

export function thresholdMarks(field: FieldDef, amount: number, numbers: Record<string, number>): number | null {
  let best: { value: number; marks: number } | null = null;
  for (const step of field.steps) {
    const value = numbers[step.id] ?? step.default;
    if (value <= amount && (best === null || value >= best.value)) best = { value, marks: step.marks };
  }
  return best ? best.marks : null;
}

export function fillFormula(formula: string, numbers: Record<string, number>): string {
  return formula.replace(/([+-])?@([a-zA-Z0-9_]+)/g, (_match, sign: string | undefined, id: string) => {
    const value = numbers[id] ?? 0;
    const applied = (sign === '-' ? -1 : 1) * value;
    return applied < 0 ? String(applied) : `+${applied}`;
  });
}
