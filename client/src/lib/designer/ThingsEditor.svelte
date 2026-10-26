<script lang="ts">
  import type { CharacterKind } from '../net/bindings/CharacterKind.ts';
  import type { FieldKind } from '../net/bindings/FieldKind.ts';
  import { designer } from './designer.svelte';
  import { blankThing, kindTitle, onSheet, SHEET_KINDS, takenIds, THING_KINDS } from './sheets';
  import ThingSettings from './ThingSettings.svelte';

  function add(kind: FieldKind) {
    if (!designer.draft) return;
    const title = THING_KINDS.find((item) => item.kind === kind)?.title ?? 'Thing';
    const field = blankThing(kind, `New ${title}`, takenIds(designer.draft));
    field.name = `New ${title.toLowerCase()}`;
    designer.draft.fields = [...designer.draft.fields, field];
    designer.expandedId = field.id;
    designer.adding = false;
  }

  function toggle(kind: CharacterKind, fieldId: string) {
    if (!designer.draft) return;
    if (onSheet(designer.draft.sheets, kind, fieldId)) designer.unplace(kind, fieldId);
    else designer.place(kind, fieldId);
  }

  function remove(fieldId: string) {
    if (!designer.draft) return;
    for (const sheet of SHEET_KINDS) designer.unplace(sheet.kind, fieldId);
    designer.draft.fields = designer.draft.fields.filter((field) => field.id !== fieldId);
    if (designer.expandedId === fieldId) designer.expandedId = null;
  }
</script>

<div class="flex flex-col gap-2">
  <p class="text-[var(--color-parchment)]/70">A thing is a value the game tracks. Turn on PC, NPC, or Adversary to put it on that sheet.</p>
  {#if designer.draft}
    {#each designer.draft.fields as field (field.id)}
      <div class="rounded-lg border border-white/10 p-2">
        <button class="text-left" type="button" onclick={() => { designer.expandedId = designer.expandedId === field.id ? null : field.id; }}>
          <span class="block text-sm">{field.name}</span>
          <span class="text-[var(--color-parchment)]/50">{kindTitle(field.kind)}</span>
        </button>
        <div class="mt-2 flex flex-wrap gap-1">
          {#each SHEET_KINDS as sheet (sheet.kind)}
            <button
              class="btn quiet px-2"
              class:active={onSheet(designer.draft.sheets, sheet.kind, field.id)}
              type="button"
              aria-pressed={onSheet(designer.draft.sheets, sheet.kind, field.id)}
              onclick={() => toggle(sheet.kind, field.id)}
            >{sheet.title}</button>
          {/each}
        </div>
        {#if designer.expandedId === field.id}
          <ThingSettings {field} />
          <button class="btn quiet mt-2" type="button" onclick={() => remove(field.id)}>Remove thing</button>
        {/if}
      </div>
    {:else}
      <p class="text-[var(--color-parchment)]/50">No things yet. Add hit points, stress, a threshold, or whatever this game tracks.</p>
    {/each}

    {#if designer.adding}
      <div class="flex flex-col gap-1">
        {#each THING_KINDS as kind (kind.kind)}
          <button class="btn quiet text-left" type="button" onclick={() => add(kind.kind)}>
            <span class="block">{kind.title}</span>
            <span class="block font-normal text-[var(--color-parchment)]/60">{kind.line}</span>
          </button>
        {/each}
        <button class="btn quiet" type="button" onclick={() => { designer.adding = false; }}>Cancel</button>
      </div>
    {:else}
      <button class="btn" type="button" onclick={() => { designer.adding = true; }}>Add a thing</button>
    {/if}
  {/if}
</div>
