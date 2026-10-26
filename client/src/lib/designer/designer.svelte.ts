// ------------------------------------------------------------
// Module: designer
// Purpose: The GM's working copy of a system definition.
//          Saving sends it to the server; nothing else sees the draft.
// ------------------------------------------------------------

import type { CharacterKind } from '../net/bindings/CharacterKind.ts';
import type { SystemDef } from '../net/bindings/SystemDef.ts';
import { table } from '../table.svelte';
import { emptySheets, insertIn, placeOn, removeFrom, systemId } from './sheets';

export type DesignerStep = 'things' | 'rolls' | 'sheets' | 'table';
export type DesignerView = 'overview' | CharacterKind;
export type DesignerNotice = { tone: 'ok' | 'error'; text: string };

class DesignerStore {
  draft = $state<SystemDef | null>(null);
  step = $state<DesignerStep>('things');
  view = $state<DesignerView>('overview');
  expandedId = $state<string | null>(null);
  adding = $state(false);
  loadedFromCampaign = $state(false);
  notice = $state<DesignerNotice | null>(null);

  get dirty(): boolean {
    if (!this.draft) return false;
    const saved = table.systems.find((system) => system.id === this.draft?.id);
    if (!saved) return true;
    return canon(this.draft) !== canon(saved);
  }

  get creating(): boolean {
    if (!this.draft) return false;
    return !table.systems.some((system) => system.id === this.draft?.id);
  }

  open(system: SystemDef) {
    this.draft = $state.snapshot(system);
    this.adding = false;
    this.expandedId = null;
    this.loadedFromCampaign = false;
    this.view = 'overview';
  }

  load(system: SystemDef, fromCampaign = false) {
    try {
      this.open(system);
      this.loadedFromCampaign = fromCampaign;
      this.view = 'overview';
      this.notice = {
        tone: 'ok',
        text: `${system.name} is loaded: ${system.fields.length} things, ${system.actions.length} rolls. Its rules are on the board.`,
      };
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error);
      console.error(error);
      this.notice = { tone: 'error', text: `Could not load ${system.name}: ${reason}` };
    }
  }

  loadCampaign(system: SystemDef) {
    this.load(system, true);
  }

  focusThing(id: string) {
    this.step = 'things';
    this.expandedId = id;
    this.adding = false;
  }

  blank(name: string) {
    const taken = table.systems.map((system) => system.id);
    const title = name.trim() || 'New system';
    this.draft = {
      id: systemId(title, taken),
      name: title,
      fields: [],
      dieTags: [],
      actions: [],
      sheets: emptySheets(),
      grid: null,
    };
    this.step = 'things';
    this.adding = false;
    this.expandedId = null;
    this.loadedFromCampaign = false;
    this.notice = null;
    this.view = 'overview';
  }

  duplicate() {
    if (!this.draft) return;
    const title = `${this.draft.name} copy`;
    const taken = table.systems.map((system) => system.id);
    this.draft = { ...$state.snapshot(this.draft), id: systemId(title, taken), name: title };
    this.adding = false;
    this.loadedFromCampaign = false;
    this.notice = null;
  }

  revert() {
    if (!this.draft) return;
    const saved = table.systems.find((system) => system.id === this.draft?.id);
    this.draft = saved ? $state.snapshot(saved) : null;
    if (!saved) this.loadedFromCampaign = false;
    this.adding = false;
    this.expandedId = null;
    this.notice = null;
  }

  place(kind: CharacterKind, fieldId: string) {
    if (!this.draft) return;
    this.draft.sheets = placeOn($state.snapshot(this.draft.sheets), kind, fieldId);
  }

  unplace(kind: CharacterKind, fieldId: string) {
    if (!this.draft) return;
    this.draft.sheets = removeFrom($state.snapshot(this.draft.sheets), kind, fieldId);
  }

  move(kind: CharacterKind, fieldId: string, sectionIndex: number, beforeId: string | null) {
    if (!this.draft) return;
    this.draft.sheets = insertIn($state.snapshot(this.draft.sheets), kind, fieldId, sectionIndex, beforeId);
  }

  removeSection(kind: CharacterKind, index: number) {
    if (!this.draft) return;
    const fields = this.draft.sheets[kind].sections[index]?.fields ?? [];
    let sheets = $state.snapshot(this.draft.sheets);
    for (const id of fields) sheets = removeFrom(sheets, kind, id);
    sheets[kind].sections = sheets[kind].sections.filter((_, item) => item !== index);
    this.draft.sheets = sheets;
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

function canon(value: unknown): string {
  return JSON.stringify(sortValue(value));
}

function sortValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortValue);
  if (value && typeof value === 'object') {
    const entries = Object.entries(value as Record<string, unknown>).sort(([left], [right]) => left.localeCompare(right));
    return Object.fromEntries(entries.map(([key, item]) => [key, sortValue(item)]));
  }
  return value;
}
