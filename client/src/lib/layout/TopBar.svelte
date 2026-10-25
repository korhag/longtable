<script lang="ts">
  import { leaveTable, send } from '../net/socket';
  import { table } from '../table.svelte';
  import ViewControls from '../ui/ViewControls.svelte';
  import { layout } from './layout.svelte';
  import { modes, type TableMode } from './modes.svelte';

  let { onZoom, onFit, onMe, onCycle, onSetGrid }: {
    onZoom: (factor: number) => void;
    onFit: () => void;
    onMe: () => void;
    onCycle: () => 'off' | 'faint' | 'strong';
    onSetGrid: (style: 'off' | 'faint' | 'strong') => void;
  } = $props();

  const gm = $derived(table.role === 'gm' && !table.viewAs);
  const tabs: { id: TableMode; name: string }[] = [
    { id: 'play', name: 'Play' },
    { id: 'map', name: 'Map' },
    { id: 'campaign', name: 'Campaign' },
    { id: 'rules', name: 'Rules' },
  ];

  function dismiss(playerId: string) {
    const player = table.players.find((item) => item.id === playerId);
    if (!gm || !player || player.id === table.playerId || player.connected) return;
    send({ type: 'removePlayer', playerId });
  }
</script>

<header class="topbar">
  <div class="min-w-0">
    <p class="truncate text-sm">{table.campaign.name || 'Campaign'}</p>
    <p class="truncate text-xs text-[var(--color-parchment)]/60">{table.scene?.name ?? 'No scene'}</p>
  </div>

  {#if gm}
    <div class="flex shrink-0 gap-1">
      {#each tabs as tab (tab.id)}
        <button class="btn quiet px-2" class:active={modes.current === tab.id} type="button" onclick={() => modes.set(tab.id)}>{tab.name}</button>
      {/each}
    </div>
  {/if}

  <div class="ml-auto flex min-w-0 items-center gap-2">
    <ViewControls {onZoom} {onFit} {onMe} {onCycle} {onSetGrid} />
    <div class="flex items-center gap-1">
      {#each table.players as player (player.id)}
        <button
          class="portrait portrait-sm"
          style={`--ring:${player.color}`}
          type="button"
          title={`${player.name}${player.role === 'gm' ? ' · GM' : ''}${!player.connected && gm && player.id !== table.playerId ? ' · click to dismiss' : ''}`}
          onclick={() => dismiss(player.id)}
        >
          {player.name.slice(0, 2).toUpperCase()}
          <span class={`seat-dot ${player.connected ? 'bg-emerald-400' : 'bg-stone-500'}`}></span>
        </button>
      {/each}
    </div>
    <button class="btn quiet px-2" type="button" onclick={() => layout.reset()}>Reset</button>
    <button class="btn quiet px-2" type="button" onclick={() => leaveTable()}>Leave</button>
  </div>
</header>
