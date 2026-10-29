<script lang="ts">
  import { send } from '../net/socket';
  import { systems } from '../systems/store.svelte';
  import { openCreation, overlaysFor } from '../../systems';
  import { table } from '../table.svelte';
  import type { Character } from '../net/bindings/Character.ts';
  import type { CharacterKind } from '../net/bindings/CharacterKind.ts';

  let making = $state(false);
  let opening = $state(false);
  let confirmId = $state('');
  let campaignName = $state('');
  let campaignSystem = $state('');
  let sceneName = $state('New scene');
  let sceneWidth = $state(30);
  let sceneHeight = $state(20);

  const guided = $derived(overlaysFor(table.campaign.systemId) != null);
  const systemName = $derived(table.systems.find((system) => system.id === table.campaign.systemId)?.name ?? 'No system');
  const saved = $derived(table.campaigns.filter((item) => item.id !== table.campaign.id));
  const confirming = $derived(confirmId !== '' && confirmId === table.campaign.id);

  function startNew() {
    making = !making;
    opening = false;
    confirmId = '';
  }

  function startOpen() {
    opening = !opening;
    making = false;
    confirmId = '';
  }

  function createCampaign() {
    send({
      type: 'createCampaign',
      name: campaignName.trim() || 'Campaign',
      systemId: campaignSystem || null,
    });
    campaignName = '';
    campaignSystem = '';
    making = false;
  }

  function openCampaign(campaignId: string) {
    send({ type: 'switchCampaign', campaignId });
    opening = false;
  }

  function deleteOpen() {
    const campaignId = table.campaign.id;
    if (!campaignId) return;
    send({ type: 'deleteCampaign', campaignId });
    confirmId = '';
  }

  const groups: { kind: CharacterKind; title: string; one: string }[] = [
    { kind: 'pc', title: 'PCs', one: 'PC' },
    { kind: 'npc', title: 'NPCs', one: 'NPC' },
    { kind: 'adversary', title: 'Adversaries', one: 'adversary' },
  ];
  const colors = ['#d4a656', '#c45c4a', '#6e9a72', '#6f8fbf', '#8d5b86', '#e7d7b6'];

  function place(character: Character) {
    const scene = table.scene;
    send({
      type: 'createToken',
      name: character.name,
      x: scene ? scene.width / 2 : 2.5,
      y: scene ? scene.height / 2 : 2.5,
      ownerId: character.ownerId,
      hidden: character.kind !== 'pc',
      visionRadius: 12,
      grantsVision: character.kind === 'pc',
      color: colors[table.tokens.length % colors.length] ?? '#d4a656',
      characterId: character.id,
    });
  }
</script>

<div class="flex flex-col gap-4 text-sm">
  <section>
    <div class="flex gap-1">
      <button class="btn flex-1" class:active={making} type="button" onclick={startNew}>New +</button>
      <button class="btn flex-1" class:active={opening} type="button" aria-expanded={opening} onclick={startOpen}>Open</button>
    </div>
    {#if making}
      <div class="mt-2 flex flex-col gap-1">
        <input class="field" bind:value={campaignName} maxlength="48" placeholder="Campaign name" aria-label="Campaign name" />
        <select class="field" bind:value={campaignSystem} aria-label="System">
          <option value="">No system</option>
          {#each table.systems as system (system.id)}
            <option value={system.id}>{system.name}</option>
          {/each}
        </select>
        <button class="btn" type="button" onclick={createCampaign}>Create</button>
      </div>
    {/if}
    {#if opening}
      <div class="mt-2 flex flex-col gap-1" aria-label="Saved campaigns">
        {#each saved as item (item.id)}
          <button class="btn quiet" type="button" onclick={() => openCampaign(item.id)}>{item.name}</button>
        {:else}
          <p class="px-1 py-1 opacity-70">No other campaigns</p>
        {/each}
      </div>
    {/if}
    <div class="mt-3">
      <input
        class="field"
        value={table.campaign.name}
        maxlength="48"
        aria-label="Open campaign name"
        onchange={(event) => send({ type: 'renameCampaign', campaignId: table.campaign.id, name: event.currentTarget.value })}
      />
      <p class="mt-1 text-[var(--color-gold)]">{systemName}</p>
    </div>
    {#if confirming}
      <p class="mt-2">Delete {table.campaign.name}? Scenes, cast, and tokens go with it.</p>
      <div class="mt-1 flex gap-1">
        <button class="btn flex-1" type="button" onclick={deleteOpen}>Delete</button>
        <button class="btn quiet flex-1" type="button" onclick={() => (confirmId = '')}>Cancel</button>
      </div>
    {:else}
      <button class="btn quiet mt-2 w-full" type="button" onclick={() => (confirmId = table.campaign.id)}>Delete</button>
    {/if}
  </section>

  <section>
    <p class="text-sm tracking-[0.18em] text-[var(--color-gold)] uppercase">Scenes</p>
    {#each table.scenes as item (item.id)}
      <div class="mt-2 flex items-center gap-1">
        <input
          class="field"
          value={item.name}
          aria-label={`Rename ${item.name}`}
          onchange={(event) => send({ type: 'renameScene', sceneId: item.id, name: event.currentTarget.value })}
        />
        {#if item.id !== table.scene?.id}
          <button class="btn quiet px-2" type="button" onclick={() => send({ type: 'switchScene', sceneId: item.id })}>Open</button>
        {:else}
          <span class="shrink-0 text-[var(--color-gold)]">Open</span>
        {/if}
        <button class="btn quiet px-2" type="button" onclick={() => send({ type: 'deleteScene', sceneId: item.id })}>Delete</button>
      </div>
    {/each}
    <div class="mt-2 flex flex-col gap-1">
      <input class="field" bind:value={sceneName} maxlength="48" aria-label="New scene name" />
      <div class="flex gap-1">
        <input class="field w-16" type="number" min="5" max="200" bind:value={sceneWidth} aria-label="Scene width" />
        <input class="field w-16" type="number" min="5" max="200" bind:value={sceneHeight} aria-label="Scene height" />
        <button
          class="btn flex-1"
          type="button"
          onclick={() => send({ type: 'createScene', name: sceneName.trim() || 'Scene', width: Math.round(sceneWidth), height: Math.round(sceneHeight) })}
        >New scene</button>
      </div>
    </div>
  </section>

  <section>
    <p class="text-sm tracking-[0.18em] text-[var(--color-gold)] uppercase">Cast</p>
    {#each groups as group (group.kind)}
      <div class="mt-3">
        <div class="flex items-center justify-between">
          <p>{group.title}</p>
          {#if guided && (group.kind === 'pc' || group.kind === 'adversary')}
            <button class="btn quiet px-2" type="button" onclick={() => openCreation(group.kind === 'pc' ? 'pc' : 'adversary')}>Create</button>
          {:else}
            <button class="btn quiet px-2" type="button" onclick={() => send({ type: 'createCharacter', name: `New ${group.one}`, kind: group.kind })}>New</button>
          {/if}
        </div>
        {#each table.characters.filter((character) => character.kind === group.kind) as character (character.id)}
          <div class="mt-1 rounded-lg border border-white/10 p-2">
            <p class="text-sm">{character.name}</p>
            {#if group.kind === 'pc'}
              <select
                class="field mt-1"
                value={character.ownerId ?? ''}
                onchange={(event) => send({
                  type: 'updateCharacter',
                  characterId: character.id,
                  name: null,
                  kind: null,
                  ownerId: event.currentTarget.value || null,
                  clearOwner: !event.currentTarget.value,
                  numbers: {},
                  texts: {},
                })}
              >
                <option value="">No owner</option>
                {#each table.players.filter((player) => player.role === 'player') as player (player.id)}
                  <option value={player.id}>{player.name}</option>
                {/each}
              </select>
            {/if}
            <div class="mt-1 flex gap-1">
              <button class="btn quiet flex-1" type="button" onclick={() => { systems.selectedId = character.id; systems.sheetOpen = true; }}>Sheet</button>
              <button class="btn quiet flex-1" type="button" onclick={() => place(character)}>Place</button>
              <button class="btn quiet px-2" type="button" onclick={() => send({ type: 'deleteCharacter', characterId: character.id })}>Delete</button>
            </div>
          </div>
        {/each}
      </div>
    {/each}
  </section>
</div>
