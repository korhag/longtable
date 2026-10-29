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
  import Glyph from './Glyph.svelte';

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

  const targetToken = $derived(table.tokens.find((token) => token.id === table.targetTokenId) ?? null);

  function signed(value: number) {
    if (value > 0) return `+${value}`;
    if (value < 0) return `${value}`;
    return '+0';
  }

  const traitShort: Record<string, string> = {
    agility: 'Agi',
    strength: 'Str',
    finesse: 'Fin',
    instinct: 'Ins',
    presence: 'Pre',
    knowledge: 'Kno',
  };

  function actionLabel(id: string, fallback: string) {
    if (!character) return fallback;
    if (id === 'damage') {
      const name = (character.texts.primary ?? '').split('·')[0]?.trim();
      const dice = (character.texts.damage ?? '').replace(/^flat:/, '').trim();
      if (name && dice) return `${name} · ${dice}`;
      if (dice) return dice;
    }
    if (id === 'atkdamage') {
      const name = (character.texts.attackname ?? '').trim() || 'Damage';
      const dice = (character.texts.atkdamage ?? '').replace(/^flat:/, '').trim();
      return dice ? `${name} · ${dice}` : name;
    }
    if (id === 'attackroll') {
      return targetToken ? `Attack ${targetToken.name}` : `Attack ${signed(character.numbers.attack ?? 0)}`;
    }
    return fallback;
  }

  function press(id: string) {
    if (id !== 'attackroll') {
      roll(id);
      return;
    }
    if (preview || !character) return;
    if (!targetToken?.characterId) {
      table.error = 'Pick a target.';
      return;
    }
    send({
      type: 'attack',
      attackerId: character.id,
      targetId: targetToken.characterId,
      plus: rollSetup.plus,
      minus: rollSetup.minus,
    });
  }

  const voiceId = $derived(preview ? null : character?.id ?? null);

  $effect(() => {
    if (!voiceId) return;
    send({ type: 'voice', characterId: voiceId });
  });

  function setPool(id: string, next: number) {
    if (!character) return;
    adjust(id, next - (character.numbers[id] ?? 0));
  }

  const strikeReady = $derived(
    Boolean(table.strike) && (!table.activeRoll || (table.landedRollId === table.activeRoll.id && table.activeRoll.contest?.blow)),
  );

</script>

{#if system && (character || gm)}
  <section class="glass pointer-events-auto rounded-2xl px-3 py-2">
    {#if !preview && strikeReady && table.strike}
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <p class="text-sm">{table.strike.targetName} takes {table.strike.step} damage, {table.strike.marks} HP.</p>
        <button class="btn quiet px-2 py-1 text-sm" type="button" onclick={() => send({ type: 'answerStrike', useArmor: false })}>Take it</button>
        <button class="btn px-2 py-1 text-sm" type="button" onclick={() => send({ type: 'answerStrike', useArmor: true })}>
          Mark armor → {table.strike.withArmor} HP
        </button>
      </div>
    {/if}
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
          <button class="btn quiet px-2 py-1 text-sm" type="button" onclick={() => openCreation('pc')}>Create</button>
        {:else}
          <button class="btn quiet px-2 py-1 text-sm" type="button" onclick={() => send({ type: 'createCharacter', name: 'New character', kind: 'pc' })}>New</button>
        {/if}
      {:else if character}
        <p class="display text-base">{character.name}</p>
      {/if}
      {#if !preview}
        <button class="btn quiet ml-auto px-2 py-1 text-sm disabled:opacity-40" type="button" disabled={!table.canUndo} onclick={() => send({ type: 'undo' })}>Undo</button>
      {/if}
      {#if character && !preview}
        <button class="btn quiet px-2 py-1 text-sm" type="button" onclick={openSheet}>Sheet</button>
      {/if}
    </div>

    {#if character && sheet}
      <div class="mt-2 flex flex-wrap items-center gap-3">
        {#each sheet.toolbar.resources as id (id)}
          {@const field = fieldOf(id)}
          {#if field && (field.kind === 'pool' || field.kind === 'number')}
            {@const value = character.numbers[id] ?? 0}
            {@const max = field.kind === 'pool' ? ceiling(field, character.numbers) : 0}
            {#if id === 'hp'}
              <div class="flex items-center gap-1 text-sm">
                <span title={field.name}><Glyph name="heart" /></span>
                <button type="button" aria-label="Decrease HP" onclick={() => adjust(id, -1)}>−</button>
                <span class="display">{value}{field.kind === 'pool' ? ` / ${max}` : ''}</span>
                <button type="button" aria-label="Increase HP" onclick={() => adjust(id, 1)}>+</button>
              </div>
            {:else if id === 'hope'}
              <div class="flex items-center gap-1 text-sm" aria-label="Hope">
                <span title={field.name}><Glyph name="hope" /></span>
                {#each Array.from({ length: Math.min(max, 12) }, (_, index) => index) as index (index)}
                  <button
                    class="slot pip"
                    class:on={index < value}
                    type="button"
                    aria-label={`Hope ${index + 1}`}
                    onclick={() => setPool(id, value === index + 1 ? index : index + 1)}
                  ></button>
                {/each}
              </div>
            {:else if id === 'stress' || id === 'armor'}
              <div class="flex items-center gap-1 text-sm" aria-label={field.name}>
                <span title={field.name}><Glyph name={id === 'armor' ? 'shield' : 'stress'} /></span>
                {#each Array.from({ length: Math.min(max, 12) }, (_, index) => index) as index (index)}
                  <button
                    class="slot box"
                    class:on={index < value}
                    type="button"
                    aria-label={`${field.name} ${index + 1}`}
                    onclick={() => setPool(id, value === index + 1 ? index : index + 1)}
                  ></button>
                {/each}
              </div>
            {:else}
              <div class="flex items-center gap-1 text-sm">
                <span>{field.name}</span>
                <button type="button" aria-label={`Decrease ${field.name}`} onclick={() => adjust(id, -1)}>−</button>
                <span class="display">{value}{field.kind === 'pool' ? ` / ${max}` : ''}</span>
                <button type="button" aria-label={`Increase ${field.name}`} onclick={() => adjust(id, 1)}>+</button>
              </div>
            {/if}
          {/if}
        {/each}
      </div>
      <div class="mt-2 flex flex-wrap items-center gap-2">
        {#each sheet.toolbar.actions as id (id)}
          {@const action = system.actions.find((item) => item.id === id)}
          {@const dice = system.fields.find((item) => item.id === id && item.kind === 'dice')}
          {#if action || dice}
            {@const full = action?.name ?? dice?.name ?? id}
            {@const short = traitShort[id]}
            <button class="btn quiet inline-flex items-center gap-1 px-2 py-1 text-sm" type="button" title={short ? full : undefined} onclick={() => press(id)}>
              {#if short}
                <Glyph name={id} />
                {short}
              {:else}
                {actionLabel(id, full)}
              {/if}
            </button>
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
