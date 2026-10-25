// ------------------------------------------------------------
// Module: designer
// Purpose: The GM's working copy of a system definition.
//          Saving sends it to the server; nothing else sees the draft.
// ------------------------------------------------------------

import type { SystemDef } from '../net/bindings/SystemDef.ts';
import { table } from '../table.svelte';

class DesignerStore {
  active = $state(false);
  draft = $state<SystemDef | null>(null);

  open(system: SystemDef) {
    this.draft = structuredClone(system);
  }

  blank(id: string, name: string) {
    this.draft = {
      id,
      name,
      fields: [],
      dieTags: [],
      actions: [],
      sheet: { sections: [] },
      toolbar: { resources: [], actions: [] },
      grid: null,
    };
  }

  revert() {
    if (!this.draft) return;
    const saved = table.systems.find((system) => system.id === this.draft?.id);
    this.draft = saved ? structuredClone(saved) : this.draft;
  }
}

export const designer = new DesignerStore();

export function moveItem<T>(items: T[], index: number, delta: number): T[] {
  const next = index + delta;
  if (next < 0 || next >= items.length) return items;
  const copy = items.slice();
  const [item] = copy.splice(index, 1);
  if (item === undefined) return items;
  copy.splice(next, 0, item);
  return copy;
}
