// ------------------------------------------------------------
// Module: rollSetup
// Purpose: The + box, the − box, and who can see a roll.
//          The dock and the character toolbar share them.
// ------------------------------------------------------------

import type { RollVisibility } from '../net/bindings/RollVisibility.ts';

class RollSetup {
  plus = $state(0);
  minus = $state(0);
  visibility = $state<RollVisibility>('public');
}

export const rollSetup = new RollSetup();
