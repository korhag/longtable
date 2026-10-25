// ------------------------------------------------------------
// Module: store
// Purpose: Which system this campaign uses, and which character
//          the toolbar is showing.
// ------------------------------------------------------------

import type { Character } from '../net/bindings/Character.ts';
import type { SystemDef } from '../net/bindings/SystemDef.ts';
import { table } from '../table.svelte';

class SystemsStore {
  selectedId = $state<string | null>(null);
  sheetOpen = $state(false);

  get active(): SystemDef | null {
    const id = table.campaign.systemId;
    if (!id) return null;
    return table.systems.find((system) => system.id === id) ?? null;
  }

  get chosen(): Character | null {
    if (this.selectedId) {
      const picked = table.characters.find((character) => character.id === this.selectedId);
      if (picked) return picked;
    }
    if (table.role !== 'gm' || table.viewAs) {
      return table.characters.find((character) => character.ownerId === table.playerId) ?? table.characters[0] ?? null;
    }
    return table.characters[0] ?? null;
  }
}

export const systems = new SystemsStore();
