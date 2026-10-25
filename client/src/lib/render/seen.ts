// ------------------------------------------------------------
// Module: seen
// Purpose: Whether a ground point is inside the player's fog.
//          The GM, and a build session, see everything.
// ------------------------------------------------------------

import { bitAt, decodeBits } from '../fog/bits';
import type { FogView } from '../net/bindings/FogView.ts';

export function seen(fog: FogView | null, x: number, y: number) {
  if (!fog || fog.mode === 'gm') return true;
  const bytes = decodeBits(fog.explored);
  const scale = fog.cellsPerSquare || 4;
  const fx = Math.floor((x + 0.5) * scale);
  const fy = Math.floor((y + 0.5) * scale);
  return bitAt(bytes, fy * fog.width + fx);
}
