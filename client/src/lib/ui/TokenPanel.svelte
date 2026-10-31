<script lang="ts">
  import { send } from '../net/socket';
  import { cellLevel, tierLabel } from '../render/levels';
  import { table } from '../table.svelte';

  let detailsOpen = $state(false);
  $effect(() => { table.selectedTokenId; detailsOpen = false; });
  const token = $derived(table.tokens.find((item) => item.id === table.selectedTokenId) ?? null);
  const target = $derived(table.tokens.find((item) => item.id === table.targetTokenId) ?? null);
  const here = $derived(token ? cellLevel(table.scene?.layout, token.x, token.y) : 0);
  const there = $derived(target ? cellLevel(table.scene?.layout, target.x, target.y) : here);
  const heightNote = $derived.by(() => {
    if (!token) return '';
    const place = tierLabel(here);
    if (!target || target.id === token.id || here === there && here === 0 && !table.scene?.layout?.levels) return place === 'On the floor' ? '' : place;
    if (here === there) return `${place}. Level with ${target.name}`;
    const steps = Math.abs(here - there) === 1 ? 'One step' : 'Two steps';
    return `${place}. ${steps} ${here > there ? 'above' : 'below'} ${target.name}`;
  });
  const gm = $derived(table.role === 'gm' && !table.viewAs);
  const ownFigure = $derived(!!token && token.ownerId === table.playerId && !table.viewAs);
  const colors = ['#d4a656', '#c45c4a', '#6e9a72', '#6f8fbf', '#8d5b86', '#e7d7b6'];
  const faces = [
    { value: 'north', label: 'North' },
    { value: 'east', label: 'East' },
    { value: 'south', label: 'South' },
    { value: 'west', label: 'West' },
  ];

  function turn(tokenId: string, facing: string) {
    send({
      type: 'updateToken',
      tokenId,
      name: null,
      ownerId: null,
      clearOwner: false,
      hidden: null,
      visionRadius: null,
      grantsVision: null,
      color: null,
      characterId: null,
      clearCharacter: false,
      setCastAsset: false,
      castAsset: null,
      facing,
    });
  }

  async function portrait(file: File, id: string) {
    const body = new FormData();
    body.append('image', file);
    const response = await fetch(`/api/tokens/${id}/image`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${table.sessionToken ?? ''}` },
      body,
    });
    if (!response.ok) table.error = await response.text();
  }
</script>

{#if token}
  <section class="selection-card" aria-label="Selected figure">
    <div class="selection-heading">
      <span class="selection-avatar" style={`--ring:${token.color}`}>{token.name.slice(0, 2).toUpperCase()}</span>
      <div class="min-w-0 flex-1"><p class="display text-lg truncate">{token.name}</p>
      <p class="selection-caption">{token.hidden ? 'Hidden from players' : 'Selected figure'}</p></div>
      <button class="selection-icon" type="button" aria-label="Deselect figure" title="Deselect figure" onclick={() => table.selectedTokenId = null}>×</button>
    </div>
    <div class="selection-quick">
    <p class="text-sm text-[var(--color-parchment)]/60">
      {token.hidden ? 'Hidden from players' : 'On the table'} · sight {token.visionRadius}
    </p>
    {#if heightNote}
      <p class="text-sm text-[var(--color-parchment)]/80">{heightNote}</p>
    {/if}

    {#if gm || ownFigure}
      <div class="mt-2 grid grid-cols-4 gap-1">
        {#each faces as face (face.value)}
          <button class="btn quiet text-sm" class:active={token.facing === face.value} type="button" onclick={() => turn(token.id, face.value)}>{face.label}</button>
        {/each}
      </div>
    {/if}
    </div>
    {#if gm || ownFigure}
      <button class="selection-details-toggle" type="button" aria-expanded={detailsOpen} onclick={() => detailsOpen = !detailsOpen}>{detailsOpen ? 'Hide details' : 'Figure details'} <span aria-hidden="true">{detailsOpen ? '⌃' : '⌄'}</span></button>
    {/if}
    {#if detailsOpen}
    <div class="selection-details">
    {#if gm}
      <label class="mt-3 block text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">Name
        <input
          class="field mt-1"
          value={token.name}
          maxlength="24"
          onchange={(event) =>
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: event.currentTarget.value,
              ownerId: null,
              clearOwner: false,
              hidden: null,
              visionRadius: null,
              grantsVision: null,
              color: null,
              characterId: null,
              clearCharacter: false,
              setCastAsset: false,
              castAsset: null,
              facing: null,
            })}
        />
      </label>
      <label class="mt-2 block text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">Owner
        <select
          class="field mt-1"
          value={token.ownerId ?? ''}
          onchange={(event) => {
            const ownerId = event.currentTarget.value;
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: null,
              ownerId: ownerId || null,
              clearOwner: ownerId === '',
              hidden: null,
              visionRadius: null,
              grantsVision: null,
              color: null,
              characterId: null,
              clearCharacter: false,
              setCastAsset: false,
              castAsset: null,
              facing: null,
            });
          }}
        >
          <option value="">No one</option>
          {#each table.players as player (player.id)}
            <option value={player.id}>{player.name}</option>
          {/each}
        </select>
        {#if token.characterId}<span class="mt-1 block text-xs normal-case tracking-normal text-[var(--color-parchment)]/60">Assigns the character sheet and its figures together.</span>{/if}
      </label>
      <label class="mt-2 block text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">Character
        <select
          class="field mt-1"
          value={token.characterId ?? ''}
          onchange={(event) => {
            const characterId = event.currentTarget.value;
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: null,
              ownerId: null,
              clearOwner: false,
              hidden: null,
              visionRadius: null,
              grantsVision: null,
              color: null,
              characterId: characterId || null,
              clearCharacter: characterId === '',
              setCastAsset: false,
              castAsset: null,
              facing: null,
            });
          }}
        >
          <option value="">None</option>
          {#each table.characters as character (character.id)}
            <option value={character.id}>{character.name}</option>
          {/each}
        </select>
      </label>
      <label class="mt-2 block text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">Vision, in squares
        <input
          class="field mt-1"
          type="number"
          min="0"
          max="80"
          value={token.visionRadius}
          onchange={(event) =>
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: null,
              ownerId: null,
              clearOwner: false,
              hidden: null,
              visionRadius: Number(event.currentTarget.value),
              grantsVision: null,
              color: null,
              characterId: null,
              clearCharacter: false,
              setCastAsset: false,
              castAsset: null,
              facing: null,
            })}
        />
      </label>
      <div class="mt-2 flex gap-1.5">
        {#each colors as color (color)}
          <button
            class="h-5 w-5 rounded-full border border-white/30"
            style={`background:${color}`}
            type="button"
            aria-label={color}
            onclick={() =>
              send({
                type: 'updateToken',
                tokenId: token.id,
                name: null,
                ownerId: null,
                clearOwner: false,
                hidden: null,
                visionRadius: null,
                grantsVision: null,
                color,
                characterId: null,
                clearCharacter: false,
              setCastAsset: false,
              castAsset: null,
              facing: null,
              })}
          ></button>
        {/each}
      </div>
      <div class="mt-3 flex flex-wrap gap-2 text-sm">
        <button
          class="btn quiet"
          type="button"
          onclick={() =>
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: null,
              ownerId: null,
              clearOwner: false,
              hidden: !token.hidden,
              visionRadius: null,
              grantsVision: null,
              color: null,
              characterId: null,
              clearCharacter: false,
              setCastAsset: false,
              castAsset: null,
              facing: null,
            })}
        >{token.hidden ? 'Show' : 'Hide'}</button>
        <button
          class="btn quiet"
          type="button"
          onclick={() =>
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: null,
              ownerId: null,
              clearOwner: false,
              hidden: null,
              visionRadius: null,
              grantsVision: !token.grantsVision,
              color: null,
              characterId: null,
              clearCharacter: false,
              setCastAsset: false,
              castAsset: null,
              facing: null,
            })}
        >{token.grantsVision ? 'Sight on' : 'Sight off'}</button>
        <button class="btn quiet" type="button" onclick={() => send({ type: 'deleteToken', tokenId: token.id })}>Remove</button>
      </div>
    {/if}

    {#if gm || token.ownerId === table.playerId}
      <label class="btn quiet mt-3 block text-center text-sm">
        Portrait
        <input
          class="hidden"
          type="file"
          accept="image/png,image/jpeg,image/webp"
          onchange={(event) => {
            const file = event.currentTarget.files?.[0];
            if (file) void portrait(file, token.id);
            event.currentTarget.value = '';
          }}
        />
      </label>
    {/if}
    </div>
    {/if}
  </section>
{/if}
