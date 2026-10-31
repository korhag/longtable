<script lang="ts">
  import { send } from '../net/socket';
  import { table } from '../table.svelte';

  let { onpreview }: { onpreview: (playerId: string | null) => void } = $props();
  let creating = $state(false);
  let name = $state('');
  let characterId = $state('');
  let pending = $state<{ name: string; ids: string[] } | null>(null);
  const players = $derived(table.players.filter(player => player.role === 'player'));
  const characters = $derived(table.characters.filter(character => character.kind === 'pc'));
  const chosen = $derived(characters.find(character => character.id === characterId));
  const assigned = $derived(table.players.find(player => player.id === chosen?.ownerId));
  const placed = $derived(table.tokens.filter(token => token.characterId === characterId));

  function openCreation() {
    let number = 1;
    while (players.some(player => player.name === `Dummy player ${number}`)) number++;
    name = `Dummy player ${number}`;
    const onMap = new Set(table.tokens.map(token => token.characterId));
    characterId = characters.find(character => !character.ownerId && onMap.has(character.id))?.id
      ?? characters.find(character => onMap.has(character.id))?.id
      ?? characters.find(character => !character.ownerId)?.id ?? characters[0]?.id ?? '';
    creating = true;
  }

  function create(preview: boolean) {
    if (pending || !name.trim() || !chosen || !table.connected) return;
    table.error = null;
    pending = { name: name.trim(), ids: players.map(player => player.id) };
    if (preview) onpreview(null);
    send({ type: 'createDummyPlayer', name: name.trim(), characterId, preview });
  }

  $effect(() => {
    if (!pending) return;
    if (players.some(player => player.dummy && player.name === pending!.name && !pending!.ids.includes(player.id))) {
      pending = null; creating = false;
    } else if (table.error || !table.connected) pending = null;
  });
</script>

<div class="player-preview flex flex-col gap-3 text-sm">
  {#if table.viewAs}
    <p role="status">Seeing as <strong>{players.find(player => player.id === table.viewAs)?.name ?? 'player'}</strong>. This preview is read-only.</p>
    <button class="btn" type="button" onclick={() => onpreview('')}>Return to GM</button>
  {:else}
    <p class="text-[var(--color-parchment)]/70">Test each player’s line of sight and personal fog in the scene currently on the table.</p>
  {/if}
  {#each players as player (player.id)}
    <div class="preview-seat">
      <div class="min-w-0 flex-1">
        <p class="truncate">{player.name}</p>
        <p class="text-xs text-[var(--color-parchment)]/60">{player.dummy ? 'Dummy player' : player.connected ? 'Player · connected' : 'Player · offline'}</p>
      </div>
      <button class="btn quiet px-2 py-1 text-xs" type="button" disabled={table.viewAs === player.id} onclick={() => onpreview(player.id)}>See as player</button>
      {#if player.dummy && !table.viewAs}
        <button class="btn quiet px-2 py-1 text-xs" type="button" aria-label={`Remove ${player.name}`} title="Remove dummy; keep its character and figures" onclick={() => send({ type: 'removePlayer', playerId: player.id })}>×</button>
      {/if}
    </div>
  {/each}
  {#if players.length === 0}<p>No player seats yet. Create a dummy to test without another browser.</p>{/if}
  {#if !table.viewAs}
    {#if creating}
      <form class="preview-form" onsubmit={event => { event.preventDefault(); create(true); }}>
        <h3 class="display text-[var(--color-gold-bright)]">Create dummy player</h3>
        <label>Player name<input class="field mt-1" bind:value={name} required maxlength="48" disabled={!!pending} /></label>
        <label>Player character
          <select class="field mt-1" bind:value={characterId} required disabled={!!pending}>
            <option value="" disabled>Choose a character</option>
            {#each characters as character (character.id)}<option value={character.id}>{character.name}{table.tokens.some(token => token.characterId === character.id) ? ' · on map' : ' · not placed'}</option>{/each}
          </select>
        </label>
        {#if characters.length === 0}<p>Create a player character or load the forest encounter first.</p>{/if}
        {#if assigned}<p class="text-[#edc78e]">This transfers {chosen?.name} from {assigned.name} to this dummy player.</p>{/if}
        {#if chosen && placed.length === 0}<p>This character has no figure in this scene. Place one to give the dummy a viewpoint.</p>
        {:else if chosen && !placed.some(token => token.grantsVision && !token.hidden && token.visionRadius > 0)}<p>This character’s figures have no active vision. Enable vision in Figure details to see the scene.</p>{/if}
        <p class="text-xs text-[var(--color-parchment)]/60">Assigns the sheet and its linked figures. Existing vision settings are used. Put map edits on the table before previewing.</p>
        <div class="flex flex-wrap gap-2">
          <button class="btn" type="submit" disabled={!!pending || !chosen || !name.trim() || !table.connected}>{pending ? 'Creating…' : 'Create & see as player'}</button>
          <button class="btn quiet" type="button" disabled={!!pending || !chosen || !name.trim() || !table.connected} onclick={() => create(false)}>Create only</button>
          <button class="btn quiet" type="button" disabled={!!pending} onclick={() => creating = false}>Cancel</button>
        </div>
      </form>
    {:else}<button class="btn quiet" type="button" onclick={openCreation}>Create dummy player</button>{/if}
  {/if}
</div>

<style>
  .player-preview { width: 100%; }
  .preview-seat { display: flex; align-items: center; gap: 8px; padding: 8px 0; border-bottom: 1px solid #b58c4d33; }
  .preview-seat button { flex-shrink: 0; }
  .preview-form { display: flex; flex-direction: column; gap: 12px; padding-top: 12px; border-top: 1px solid #b58c4d66; }
</style>
