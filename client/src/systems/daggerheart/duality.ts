// ------------------------------------------------------------
// Module: duality
// Purpose: The Hope and Fear button for Daggerheart only.
// ------------------------------------------------------------

export type DualityDie = 12 | 20;
export type DualityEdge = 'none' | 'advantage' | 'disadvantage';

export interface DualityConfig {
  hope: DualityDie;
  fear: DualityDie;
  edge: DualityEdge;
}

export const DEFAULT_DUALITY: DualityConfig = {
  hope: 12,
  fear: 12,
  edge: 'none',
};

export function dualityFormula(config: DualityConfig): string {
  let text = `1d${config.hope}h+1d${config.fear}f`;
  if (config.edge === 'advantage') text += '+1d6';
  if (config.edge === 'disadvantage') text += '-1d6';
  return text;
}

export function dualityCaption(config: DualityConfig): string {
  const edge = config.edge === 'advantage' ? ' +d6' : config.edge === 'disadvantage' ? ' −d6' : '';
  return `d${config.hope} · d${config.fear}${edge}`;
}
