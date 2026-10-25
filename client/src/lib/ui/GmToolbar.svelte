<script lang="ts">
  import { send } from '../net/socket';
  import { table, type Tool } from '../table.svelte';

  let tokenName = $state('Hero');

  const players = $derived(table.players.filter((player) => player.role === 'player'));
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

  <p class="mt-3 text-xs text-[var(--color-parchment)]/55">Import a map or start a blank one from the Map tool. Click a door on the map to open it.</p>

  <div class="mt-3 flex gap-1.5">
    <input class="field" bind:value={tokenName} maxlength="24" placeholder="Token name" />
    <button class="btn text-xs" type="button" onclick={addToken}>Add</button>
  </div>
</div>
