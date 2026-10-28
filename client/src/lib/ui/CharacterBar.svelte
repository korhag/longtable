<script lang="ts">
  import { rollSetup } from '../dice/rollSetup.svelte';
  import type { Character } from '../net/bindings/Character.ts';
  import type { FieldDef } from '../net/bindings/FieldDef.ts';
  import type { SystemDef } from '../net/bindings/SystemDef.ts';
  import { send } from '../net/socket';
  import { layout } from '../layout/layout.svelte';
  import Panel from '../layout/Panel.svelte';
  import { ceiling } from '../systems/fields';
  import { systems } from '../systems/store.svelte';
  import { openCreation, overlaysFor, sheetFor } from '../../systems';
  import { table } from '../table.svelte';
  import CharacterSheet from './CharacterSheet.svelte';

  let { preview = null }: { preview?: { character: Character; system: SystemDef } | null } = $props();

  const character = $derived(preview?.character ?? systems.chosen);
  const system = $derived(preview?.system ?? systems.active);
  const gm = $derived(!preview && table.role === 'gm' && !table.viewAs);
  const sheet = $derived(character && system ? system.sheets[character.kind] : null);

  function fieldOf(id: string): FieldDef | undefined {
    return system?.fields.find((item) => item.id === id);
  }

  function adjust(field: string, delta: number) {
    if (preview || !character) return;
    send({ type: 'adjustResource', characterId: character.id, field, delta });
  }

  const Fancy = $derived(character && system ? sheetFor(system.id, character.kind) : null);
  const guided = $derived(overlaysFor(table.campaign.systemId) != null);

  function openSheet() {
    systems.sheetOpen = !systems.sheetOpen;
    if (!systems.sheetOpen) return;
    const place = layout.place('sheet');
    layout.patch('sheet', {
      floating: true,
      collapsed: false,
      w: Math.max(place.w, Fancy ? 760 : place.w),
      h: Math.max(place.h, Fancy ? 680 : place.h),
    });
  }

  function roll(actionId: string) {
    if (preview || !character) return;
    send({
      type: 'rollAction',
      characterId: character.id,
      actionId,
      visibility: rollSetup.visibility,
      plus: rollSetup.plus,
      minus: rollSetup.minus,
    });
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
        {#if guided}
          <button class="btn quiet px-2 py-1 text-[12px]" type="button" onclick={() => openCreation('pc')}>Create</button>
        {:else}
          <button class="btn quiet px-2 py-1 text-[12px]" type="button" onclick={() => send({ type: 'createCharacter', name: 'New character', kind: 'pc' })}>New</button>
        {/if}
      {:else if character}
        <p class="display text-base">{character.name}</p>
      {/if}
      {#if character && !preview}
        <button class="btn quiet ml-auto px-2 py-1 text-[12px]" type="button" onclick={openSheet}>Sheet</button>
      {/if}
    </div>

    {#if character && sheet}
      <div class="mt-2 flex flex-wrap items-center gap-2">
        {#each sheet.toolbar.resources as id (id)}
          {@const field = fieldOf(id)}
          {#if field && (field.kind === 'pool' || field.kind === 'number')}
            <div class="flex items-center gap-1 rounded-lg border border-[rgba(212,166,86,0.3)] px-1.5 py-1 text-[12px]">
              <span>{field.name}</span>
              <button type="button" aria-label={`Decrease ${field.name}`} onclick={() => adjust(id, -1)}>−</button>
              <span class="display text-sm">{character.numbers[id] ?? 0}</span>
              {#if field.kind === 'pool'}<span class="text-[var(--color-parchment)]/45">/ {ceiling(field, character.numbers)}</span>{/if}
              <button type="button" aria-label={`Increase ${field.name}`} onclick={() => adjust(id, 1)}>+</button>
            </div>
          {/if}
        {/each}
        {#each sheet.toolbar.actions as id (id)}
          {@const action = system.actions.find((item) => item.id === id)}
          {@const dice = system.fields.find((item) => item.id === id && item.kind === 'dice')}
          {#if action || dice}
            <button class="btn quiet px-2 py-1 text-[12px]" type="button" onclick={() => roll(id)}>{action?.name ?? dice?.name}</button>
          {/if}
        {/each}
      </div>
    {/if}
  </section>
{/if}

{#if !preview && systems.sheetOpen && character && system}
  <Panel id="sheet" title={character.name} onclose={() => (systems.sheetOpen = false)}>
    {#if Fancy}
      <Fancy {character} />
    {:else}
      <CharacterSheet {character} {system} />
    {/if}
  </Panel>
{/if}
