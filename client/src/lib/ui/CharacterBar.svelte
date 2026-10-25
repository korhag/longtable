<script lang="ts">
  import { rollSetup } from '../dice/rollSetup.svelte';
  import type { Character } from '../net/bindings/Character.ts';
  import type { FieldDef } from '../net/bindings/FieldDef.ts';
  import type { SystemDef } from '../net/bindings/SystemDef.ts';
  import { send } from '../net/socket';
  import { systems } from '../systems/store.svelte';
  import { table } from '../table.svelte';
  import CharacterSheet from './CharacterSheet.svelte';

  let { preview = null }: { preview?: { character: Character; system: SystemDef } | null } = $props();

  const character = $derived(preview?.character ?? systems.chosen);
  const system = $derived(preview?.system ?? systems.active);
  const gm = $derived(!preview && table.role === 'gm' && !table.viewAs);

  function ceiling(field: FieldDef) {
    if (!character) return 0;
    if (field.maxFrom) return Math.max(0, character.numbers[field.maxFrom] ?? 0);
    return field.max ?? 1000;
  }

  function adjust(field: string, delta: number) {
    if (preview || !character) return;
    send({ type: 'adjustResource', characterId: character.id, field, delta });
  }
</script>

{#if system && (character || gm)}
  <section class="glass pointer-events-auto rounded-2xl px-3 py-2">
    <div class="flex items-center gap-2">
      {#if gm}
        <select
          class="field max-w-40"
          value={character?.id ?? ''}
          onchange={(event) => (systems.selectedId = event.currentTarget.value || null)}
        >
          <option value="">Character</option>
          {#each table.characters as item (item.id)}
            <option value={item.id}>{item.name}</option>
          {/each}
        </select>
        <button class="btn quiet px-2 py-1 text-[11px]" type="button" onclick={() => send({ type: 'createCharacter', name: 'New character', kind: 'pc' })}>New</button>
      {:else if character}
        <p class="display text-base">{character.name}</p>
      {/if}
      {#if character && !preview}
        <button class="btn quiet ml-auto px-2 py-1 text-[11px]" type="button" onclick={() => (systems.sheetOpen = !systems.sheetOpen)}>Sheet</button>
      {/if}
    </div>

    {#if character}
      <div class="mt-2 flex flex-wrap items-center gap-2">
        {#each system.toolbar.resources as id (id)}
          {@const field = system.fields.find((item) => item.id === id)}
          {#if field}
            <div class="flex items-center gap-1 rounded-lg border border-[rgba(212,166,86,0.3)] px-1.5 py-1 text-[11px]">
              <span>{field.name}</span>
              <button type="button" aria-label={`Decrease ${field.name}`} onclick={() => adjust(id, -1)}>−</button>
              <span class="display text-sm">{character.numbers[id] ?? 0}</span>
              {#if field.kind !== 'number'}<span class="text-[var(--color-parchment)]/45">/ {ceiling(field)}</span>{/if}
              <button type="button" aria-label={`Increase ${field.name}`} onclick={() => adjust(id, 1)}>+</button>
            </div>
          {/if}
        {/each}
        {#each system.toolbar.actions as id (id)}
          {@const action = system.actions.find((item) => item.id === id)}
          {#if action}
            <button
              class="btn quiet px-2 py-1 text-[11px]"
              type="button"
              onclick={() => {
                if (preview || !character) return;
                send({
                  type: 'rollAction',
                  characterId: character.id,
                  actionId: id,
                  visibility: rollSetup.visibility,
                  plus: rollSetup.plus,
                  minus: rollSetup.minus,
                });
              }}
            >{action.name}</button>
          {/if}
        {/each}
      </div>
    {/if}
  </section>
{/if}

{#if !preview && systems.sheetOpen && character && system}
  <div class="pointer-events-auto fixed top-16 bottom-24 left-4 z-30 w-[min(40rem,calc(100vw-28rem))] overflow-auto rounded-2xl border border-[rgba(212,166,86,0.35)] bg-[#1c140e] p-4 shadow-2xl">
    <div class="mb-3 flex items-center justify-between">
      <p class="display text-xl">{character.name}</p>
      <button class="btn quiet px-2 py-1 text-xs" type="button" onclick={() => (systems.sheetOpen = false)}>Close</button>
    </div>
    <CharacterSheet {character} {system} />
  </div>
{/if}
