// ------------------------------------------------------------
// Module: modes
// Purpose: The GM's four tools. Map opens the editor.
//          Leaving Map closes it. Players never see the tabs.
// ------------------------------------------------------------

import { designer } from '../designer/designer.svelte';
import { editor } from '../editor/draft.svelte';
import { systems } from '../systems/store.svelte';

export type TableMode = 'play' | 'map' | 'campaign' | 'rules';
export type AppMode = 'play' | 'edit' | 'design';

export function appMode(tab: TableMode): AppMode {
  if (tab === 'play') return 'play';
  if (tab === 'rules') return 'design';
  return 'edit';
}

class ModeStore {
  current = $state<TableMode>('play');

  set(next: TableMode) {
    if (this.current === next) return;
    if (this.current === 'map') editor.leave();
    if (next === 'map') void editor.enter();
    this.current = next;
    if (next === 'rules' && !designer.draft && systems.active) designer.loadCampaign(systems.active);
  }
}

export const modes = new ModeStore();
