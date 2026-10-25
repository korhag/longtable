// ------------------------------------------------------------
// Module: measure
// Purpose: Turn a drag across the grid into a distance.
//          Alternate diagonals count 5, 10, 5, 10.
// ------------------------------------------------------------

import type { GridRule } from '../net/bindings/GridRule.ts';

export function squareDistance(dx: number, dy: number, diagonal: GridRule['diagonal']): number {
  const adx = Math.abs(dx);
  const ady = Math.abs(dy);
  if (diagonal === 'euclidean') return Math.hypot(adx, ady);
  if (diagonal === 'square') return Math.max(adx, ady);
  const diagonalSteps = Math.min(adx, ady);
  const straight = Math.abs(adx - ady);
  return straight + diagonalSteps + Math.floor(diagonalSteps / 2);
}

export function measure(dx: number, dy: number, grid: GridRule | null): { squares: number; text: string; range: string | null } {
  const squares = squareDistance(dx, dy, grid?.diagonal ?? 'alternate');
  const perSquare = grid?.perSquare ?? 5;
  const distance = Math.round(squares * perSquare);
  const unit = grid?.unit ?? 'ft';
  const text = unit ? `${distance} ${unit}` : `${distance}`;
  const range = grid?.ranges.find((band) => squares <= band.upTo)?.name ?? null;
  return { squares, text, range };
}
