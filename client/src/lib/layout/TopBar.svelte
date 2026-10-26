<script lang="ts">
  import { designer } from '../designer/designer.svelte';
  import { leaveTable, send } from '../net/socket';
  import { systems } from '../systems/store.svelte';
  import { table } from '../table.svelte';
  import ViewControls from '../ui/ViewControls.svelte';
  import { VERSION, VERSION_NOTE } from '../version';
  import { layout } from './layout.svelte';
  import { appMode, modes, type AppMode, type TableMode } from './modes.svelte';

  let { onZoom, onFit, onMe, onCycle, onSetGrid }: {
    onZoom: (factor: number) => void;
    onFit: () => void;
    onMe: () => void;
    onCycle: () => 'off' | 'faint' | 'strong';
    onSetGrid: (style: 'off' | 'faint' | 'strong') => void;
  } = $props();

  const gm = $derived(table.role === 'gm' && !table.viewAs);
  const mode = $derived<AppMode>(gm ? appMode(modes.current) : 'play');
  const sceneName = $derived(table.scene?.name ?? 'No scene');
  const systemName = $derived(systems.active?.name ?? 'No system');
  const expectation = $derived.by(() => {
    if (mode === 'play') return 'Running the table.';
    if (mode === 'edit') return 'Preparing maps, scenes, and the cast.';
    const name = designer.draft?.name;
    const base = name ? `Editing the ${name} rule set` : 'Choose a rule set to edit';
    return designer.dirty ? `${base} · unsaved` : `${base}.`;
  });

  const editTabs: { id: TableMode; name: string }[] = [
    { id: 'map', name: 'Map' },
    { id: 'campaign', name: 'Campaign' },
  ];

  function dismiss(playerId: string) {
    const player = table.players.find((item) => item.id === playerId);
    if (!gm || !player || player.id === table.playerId || player.connected) return;
    send({ type: 'removePlayer', playerId });
  }
</script>

<header class="topbar" data-mode={mode}>
  <div class="min-w-0">
    <div class="flex min-w-0 items-center gap-2">
      <span class="mode-badge {mode}">{mode}</span>
      <p class="truncate text-sm">{expectation}</p>
    </div>
    <p class="truncate text-xs text-[var(--color-parchment)]/60">
      {table.campaign.name || 'Campaign'} · {sceneName} · {systemName} ·
      <span title={VERSION_NOTE}>v{VERSION}</span>
    </p>
  </div>

  {#if gm}
    <div class="flex shrink-0 items-center gap-2">
      <button class="btn quiet px-2" class:active={modes.current === 'play'} type="button" onclick={() => modes.set('play')}>Play</button>
      <span class="text-[11px] tracking-[0.14em] text-[var(--color-parchment)]/45 uppercase">Edit</span>
      {#each editTabs as tab (tab.id)}
        <button class="btn quiet px-2" class:active={modes.current === tab.id} type="button" onclick={() => modes.set(tab.id)}>{tab.name}</button>
      {/each}
      <span class="text-[11px] tracking-[0.14em] text-[var(--color-parchment)]/45 uppercase">Design</span>
      <button class="btn quiet px-2" class:active={modes.current === 'rules'} type="button" onclick={() => modes.set('rules')}>Rules</button>
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
