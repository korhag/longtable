// ------------------------------------------------------------
// Module: rollText
// Purpose: Attach the + and − boxes to a formula without
//          producing a sign the parser would reject.
// ------------------------------------------------------------

export function clampModifier(text: string): number {
  const value = Number(text);
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(1000, Math.trunc(value)));
}

export function applyModifiers(formula: string, plus: number, minus: number): string {
  let text = formula.trim();
  if (plus > 0) text += `+${plus}`;
  if (minus > 0) text += `-${minus}`;
  return text;
}
