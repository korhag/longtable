<script lang="ts">
  import { send } from '../net/socket';
  import { openCreation, overlaysFor } from '../../systems';
  import { table, type Tool } from '../table.svelte';

  let tokenName = $state('Hero');

  const players = $derived(table.players.filter((player) => player.role === 'player'));
  const guided = $derived(overlaysFor(table.campaign.systemId) != null);
  const colors = ['#d4a656', '#c45c4a', '#6e9a72', '#6f8fbf', '#8d5b86', '#e7d7b6'];

  function tool(next: Tool) {
    table.tool = table.tool === next ? 'select' : next;
  }

  function addToken() {
    const scene = table.scene;
    send({
      type: 'createToken',
      name: tokenName.trim() || 'Token',
      x: scene ? scene.width / 2 : 2.5,
      y: scene ? scene.height / 2 : 2.5,
      ownerId: null,
      hidden: false,
      visionRadius: 12,
      grantsVision: true,
      color: colors[table.tokens.length % colors.length] ?? '#d4a656',
      characterId: null,
    });
  }

  function place(character: (typeof table.characters)[number]) {
    const scene = table.scene;
    const index = table.tokens.length;
    send({
      type: 'createToken',
      name: character.name,
      x: (scene ? scene.width / 2 : 2.5) + (index % 5) - 2,
      y: (scene ? scene.height / 2 : 2.5) + Math.floor(index / 5),
      ownerId: character.ownerId,
      hidden: character.kind !== 'pc',
      visionRadius: character.kind === 'pc' ? 12 : 8,
      grantsVision: character.kind === 'pc',
      color: character.kind === 'adversary' ? '#c45c4a' : (colors[index % colors.length] ?? '#d4a656'),
      characterId: character.id,
    });
    table.tool = 'select';
  }
</script>

<div class="flex flex-col gap-1 text-xs">
  <p class="display text-sm text-[var(--color-gold-bright)]">{table.scene?.name ?? 'Empty table'}</p>
  <p class="mt-1 text-[11px] text-[var(--color-parchment)]/60">
    {table.viewAs ? 'Looking through a player’s eyes' : 'Drag empty ground to pan. Wheel zooms.'}
  </p>

  <div class="mt-3 grid grid-cols-3 gap-1.5">
    <button class="btn quiet text-xs" class:active={table.tool === 'select'} type="button" onclick={() => (table.tool = 'select')}>Move</button>
    <button class="btn quiet text-xs" class:active={table.tool === 'reveal'} type="button" disabled={!!table.viewAs} onclick={() => tool('reveal')}>Reveal</button>
    <button class="btn quiet text-xs" class:active={table.tool === 'hide'} type="button" disabled={!!table.viewAs} onclick={() => tool('hide')}>Hide</button>
  </div>

  <label class="mt-3 block text-[10px] tracking-wide text-[var(--color-gold)]/70 uppercase">See as
    <select
      class="field mt-1"
      value={table.viewAs ?? ''}
      onchange={(event) => {
        const playerId = event.currentTarget.value;
        send({ type: 'viewAs', playerId: playerId || null });
      }}
    >
      <option value="">Yourself, the GM</option>
      {#each players as player (player.id)}
        <option value={player.id}>{player.name}</option>
      {/each}
    </select>
  </label>

  <p class="mt-3 text-xs text-[var(--color-parchment)]/55">Move is on. Drag a player or an adversary. The range band shows while you drag. Build walls and lights in Map, then put that map on the table.</p>

  {#if guided}
    <div class="mt-3 flex gap-1.5">
      <button class="btn flex-1 text-xs" type="button" onclick={() => openCreation('pc')}>Character</button>
      <button class="btn quiet flex-1 text-xs" type="button" onclick={() => openCreation('adversary')}>Adversary</button>
    </div>
  {/if}

  {#if table.characters.length > 0}
    <p class="mt-3 text-[10px] tracking-wide text-[var(--color-gold)]/70 uppercase">Cast</p>
    {#each table.characters as character (character.id)}
      {@const token = table.tokens.find((item) => item.characterId === character.id)}
      <div class="mt-1 flex items-center gap-1">
        <button
          class="btn quiet min-w-0 flex-1 truncate text-left text-xs"
          type="button"
          onclick={() => { if (token) table.selectedTokenId = token.id; table.tool = 'select'; }}
        >{character.name}</button>
        <button class="btn quiet px-2 text-xs" type="button" onclick={() => place(character)}>{token ? 'Again' : 'Place'}</button>
      </div>
    {/each}
  {/if}

  <div class="mt-3 flex gap-1.5">
    <input class="field" bind:value={tokenName} maxlength="24" placeholder="Token name" />
    <button class="btn text-xs" type="button" onclick={addToken}>Add</button>
  </div>
</div>
