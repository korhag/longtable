// ------------------------------------------------------------
// Module: modes
// Purpose: The GM's four tools. Map opens the editor.
//          Leaving Map closes it. Players never see the tabs.
// ------------------------------------------------------------

import { designer } from '../designer/designer.svelte';
import { editor } from '../editor/draft.svelte';

export type TableMode = 'play' | 'map' | 'campaign' | 'rules';

class ModeStore {
  current = $state<TableMode>('play');

  set(next: TableMode) {
    if (this.current === next) return;
    if (this.current === 'map') editor.leave();
    if (next === 'map') void editor.enter();
    designer.active = next === 'rules';
    this.current = next;
  }
}

export const modes = new ModeStore();
