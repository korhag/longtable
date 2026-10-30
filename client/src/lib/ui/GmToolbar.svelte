<script lang="ts">
  import { send } from '../net/socket';
  import { openCreation, overlaysFor } from '../../systems';
  import { table, type Tool } from '../table.svelte';

  let tokenName = $state('Hero');

  const players = $derived(table.players.filter((player) => player.role === 'player'));
  const guided = $derived(overlaysFor(table.campaign.systemId) != null);

  function tool(next: Tool) {
    table.tool = table.tool === next ? 'select' : next;
  }

  function addToken() {
    table.placingId = null;
    table.placingName = tokenName.trim() || 'Token';
    table.placingToken = !table.placingToken;
  }

  function place(character: (typeof table.characters)[number]) {
    table.placingToken = false;
    table.placingId = table.placingId === character.id ? null : character.id;
  }
</script>

<div class="flex flex-col gap-1 text-sm">
  <p class="display text-sm text-[var(--color-gold-bright)]">{table.scene?.name ?? 'Empty table'}</p>
  <p class="mt-1 text-sm text-[var(--color-parchment)]/60">
    {table.viewAs ? 'Looking through a player’s eyes' : 'Drag empty ground to pan. Wheel zooms.'}
  </p>

  <div class="mt-3 grid grid-cols-3 gap-1.5">
    <button class="btn quiet text-sm" class:active={table.tool === 'select'} type="button" onclick={() => (table.tool = 'select')}>Move</button>
    <button class="btn quiet text-sm" class:active={table.tool === 'reveal'} type="button" disabled={!!table.viewAs} onclick={() => tool('reveal')}>Reveal</button>
    <button class="btn quiet text-sm" class:active={table.tool === 'hide'} type="button" disabled={!!table.viewAs} onclick={() => tool('hide')}>Hide</button>
  </div>

  <label class="mt-3 block text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">See as
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

  <p class="mt-3 text-sm text-[var(--color-parchment)]/55">Move is on. Drag a player or an adversary. The range band shows while you drag. Build walls and lights in Map, then put that map on the table.</p>

  {#if guided}
    <div class="mt-3 flex gap-1.5">
      <button class="btn flex-1 text-sm" type="button" onclick={() => openCreation('pc')}>Character</button>
      <button class="btn quiet flex-1 text-sm" type="button" onclick={() => openCreation('adversary')}>Adversary</button>
    </div>
  {/if}

  {#if table.characters.length > 0}
    <p class="mt-3 text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">Cast</p>
    {#each table.characters as character (character.id)}
      {@const token = table.tokens.find((item) => item.characterId === character.id)}
      <div class="mt-1 flex items-center gap-1">
        <button
          class="btn quiet min-w-0 flex-1 truncate text-left text-sm"
          type="button"
          onclick={() => { if (token) table.selectedTokenId = token.id; table.tool = 'select'; }}
        >{character.name}</button>
        <button class="btn quiet px-2 text-sm" type="button" onclick={() => place(character)}>{table.placingId === character.id ? 'Cancel' : token ? 'Again' : 'Place'}</button>
      </div>
    {/each}
  {/if}

  <div class="mt-3 flex gap-1.5">
    <input class="field" bind:value={tokenName} maxlength="24" placeholder="Token name" />
    <button class="btn text-sm" type="button" onclick={addToken}>{table.placingToken ? 'Cancel' : 'Add'}</button>
  </div>
  {#if table.placingId || table.placingToken}
    <p class="mt-2 text-sm text-[var(--color-parchment)]/70">Click the ground to place them. Right-click cancels.</p>
  {/if}
</div>
