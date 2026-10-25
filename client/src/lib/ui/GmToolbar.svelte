<script lang="ts">
  import { send } from '../net/socket';
  import { table, type Tool } from '../table.svelte';

  let width = $state(30);
  let height = $state(20);
  let tokenName = $state('Hero');
  let fileInput: HTMLInputElement;

  const players = $derived(table.players.filter((player) => player.role === 'player'));
  const colors = ['#d4a656', '#c45c4a', '#6e9a72', '#6f8fbf', '#8d5b86', '#e7d7b6'];

  function tool(next: Tool) {
    table.tool = table.tool === next ? 'select' : next;
  }

  async function importMap(file: File) {
    const body = new FormData();
    body.append('map', file);
    const response = await fetch('/api/maps/import', {
      method: 'POST',
      headers: { Authorization: `Bearer ${table.sessionToken ?? ''}` },
      body,
    });
    if (!response.ok) table.error = await response.text();
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
    });
  }
</script>

<div class="glass pointer-events-auto fixed top-20 left-4 z-10 w-64 rounded-2xl p-3">
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

  <div class="mt-3 flex gap-2">
    <button class="btn quiet flex-1 text-xs" type="button" onclick={() => fileInput.click()}>Import map</button>
    <input
      bind:this={fileInput}
      class="hidden"
      type="file"
      accept=".dd2vtt,.uvtt,.json,application/json"
      onchange={(event) => {
        const file = event.currentTarget.files?.[0];
        if (file) void importMap(file);
        event.currentTarget.value = '';
      }}
    />
  </div>
  <p class="mt-1 text-[10px] leading-snug text-[var(--color-parchment)]/50">Dungeon Alchemist Universal VTT (.dd2vtt). Click a door on the map to open it.</p>

  <div class="mt-3 flex items-end gap-1.5">
    <label class="text-[10px] tracking-wide text-[var(--color-gold)]/70 uppercase">W
      <input class="field mt-1 w-16" type="number" min="5" max="200" bind:value={width} />
    </label>
    <label class="text-[10px] tracking-wide text-[var(--color-gold)]/70 uppercase">H
      <input class="field mt-1 w-16" type="number" min="5" max="200" bind:value={height} />
    </label>
    <button
      class="btn quiet mb-0.5 text-xs"
      type="button"
      onclick={() => send({ type: 'blankMap', width: Math.round(width), height: Math.round(height) })}
    >Blank</button>
  </div>

  <div class="mt-3 flex gap-1.5">
    <input class="field" bind:value={tokenName} maxlength="24" placeholder="Token name" />
    <button class="btn text-xs" type="button" onclick={addToken}>Add</button>
  </div>
</div>
