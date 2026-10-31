// ------------------------------------------------------------
// Module: rollSetup
// Purpose: The + box, the − box, and who can see a roll.
//          The dock and the character toolbar share them.
// ------------------------------------------------------------

import type { RollVisibility } from '../net/bindings/RollVisibility.ts';
import type { AttackEdge } from '../net/bindings/AttackEdge.ts';

class RollSetup {
  plus = $state(0);
  minus = $state(0);
  attackEdge = $state<AttackEdge>('none');
  visibility = $state<RollVisibility>('public');
}

export const rollSetup = new RollSetup();
