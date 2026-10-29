// ------------------------------------------------------------
// Module: rings
// Purpose: Which color a token ring wears. The GM reads white,
//          gray, and red. A player keeps their own color, and
//          the GM's cast stays gray until it is targeted.
// ------------------------------------------------------------

export type CastTone = 'player' | 'other';
export type RingMark = 'control' | 'target' | 'hover' | null;

const QUIET = '#6e675e';
const WHITE = '#f4f1ea';
const TARGET = '#e23b32';

export interface CastPaint {
  tone: CastTone;
  color: string;
}

export interface RingPaint {
  color: string;
  scale: number;
  halo: string | null;
  ticks: boolean;
}

export function castFor(
  token: { characterId: string | null; ownerId: string | null },
  characters: { id: string; kind: string; ownerId: string | null }[],
  players: { id: string; color: string }[],
): CastPaint {
  const character = characters.find((item) => item.id === token.characterId);
  const ownerId = character?.ownerId ?? token.ownerId;
  const owner = players.find((player) => player.id === ownerId);
  if (character?.kind === 'pc' && owner) return { tone: 'player', color: owner.color };
  return { tone: 'other', color: QUIET };
}

export function ringPaint(tone: CastTone, color: string, mark: RingMark, gm: boolean): RingPaint {
  if (gm) {
    if (mark === 'control') return { color: WHITE, scale: 1.34, halo: null, ticks: false };
    if (mark === 'target') return { color: TARGET, scale: 1.22, halo: null, ticks: false };
    if (mark === 'hover') return { color: brighten(QUIET), scale: 1.16, halo: null, ticks: false };
    return { color: QUIET, scale: 1, halo: null, ticks: false };
  }
  if (tone === 'other') {
    if (mark === 'target') return { color: TARGET, scale: 1.18, halo: null, ticks: true };
    if (mark === 'hover') return { color: brighten(QUIET), scale: 1.12, halo: null, ticks: false };
    return { color: QUIET, scale: mark === 'control' ? 1.2 : 1, halo: null, ticks: false };
  }
  if (mark === 'control') return { color, scale: 1.34, halo: null, ticks: false };
  if (mark === 'target') return { color, scale: 1.12, halo: brighten(color), ticks: false };
  if (mark === 'hover') return { color: brighten(color), scale: 1.22, halo: null, ticks: false };
  return { color, scale: 1, halo: null, ticks: false };
}

export function brighten(color: string): string {
  const hex = color.replace('#', '');
  if (hex.length !== 6) return color;
  const mix = (index: number) => Math.round(Number.parseInt(hex.slice(index, index + 2), 16) * 0.55 + 255 * 0.45);
  return `#${[mix(0), mix(2), mix(4)].map((part) => part.toString(16).padStart(2, '0')).join('')}`;
}
