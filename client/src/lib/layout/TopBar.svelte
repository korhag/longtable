<script lang="ts">
  import Soundboard from '../ui/Soundboard.svelte';
  import { onDestroy, onMount } from 'svelte';
  import EnvironmentControls from '../ui/EnvironmentControls.svelte';
  import type { EnvironmentLighting } from '../net/bindings/EnvironmentLighting';
  import { editor } from '../editor/draft.svelte';

  import { designer } from '../designer/designer.svelte';
  import type { PlayerInfo } from '../net/bindings/PlayerInfo.ts';
  import { leaveTable, send } from '../net/socket';
  import { systems } from '../systems/store.svelte';
  import { table } from '../table.svelte';
  import ViewControls from '../ui/ViewControls.svelte';
  import type { Anchor } from '../ui/portal';
  import { VERSION, VERSION_NOTE } from '../version';
  import { account } from '../account.svelte';
  import { chrome } from '../chrome.svelte';
  import { layout } from './layout.svelte';
  import { appMode, modes, type AppMode, type TableMode } from './modes.svelte';
  import SeatMenu from './SeatMenu.svelte';

  let { onZoom, onFit, onMe, onCycle, onSetGrid }: {
    onZoom: (factor: number) => void;
    onFit: () => void;
    onMe: () => void;
    onCycle: () => 'off' | 'faint' | 'strong';
    onSetGrid: (style: 'off' | 'faint' | 'strong') => void;
  } = $props();

  let headerEl: HTMLElement | undefined;
  let seatTrigger: HTMLElement | null = null;

  function closeMenus(except: HTMLDetailsElement | null = null, restoreFocus = false) {
    let trigger: HTMLElement | null = null;
    for (const menu of headerEl?.querySelectorAll<HTMLDetailsElement>('.topbar-menu[open]') ?? []) {
      if (menu === except) continue;
      menu.open = false;
      trigger = menu.querySelector<HTMLElement>('summary');
    }
    if (restoreFocus) trigger?.focus();
  }

  function menuToggled(event: Event) {
    const menu = event.currentTarget as HTMLDetailsElement;
    if (menu.open) closeMenus(menu);
  }

  function closeSeat(restoreFocus = false) {
    seatMenu = null;
    if (restoreFocus && seatTrigger?.isConnected) seatTrigger.focus();
  }

  onMount(() => {
    const outside = (event: Event) => {
      // The seat's child menu is portalled to the document. Its own shade
      // dismisses it, so a click on its actions must not unmount them first.
      if (seatMenu) return;
      const open = headerEl?.querySelector<HTMLDetailsElement>('.topbar-menu[open]');
      if (open && event.target instanceof Node && !open.contains(event.target)) closeMenus();
    };
    const escape = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.key !== 'Escape') return;
      if (seatMenu) {
        event.preventDefault(); event.stopPropagation(); closeSeat(true);
      } else if (headerEl?.querySelector('.topbar-menu[open]')) {
        event.preventDefault(); event.stopPropagation(); closeMenus(null, true);
      }
    };
    document.addEventListener('pointerdown', outside, true);
    document.addEventListener('focusin', outside);
    document.addEventListener('keydown', escape, true);
    return () => {
      document.removeEventListener('pointerdown', outside, true);
      document.removeEventListener('focusin', outside);
      document.removeEventListener('keydown', escape, true);
    };
  });

  let lightingTimer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => clearTimeout(lightingTimer));
  function changeEnvironment(environment: EnvironmentLighting, commit: boolean) {
    clearTimeout(lightingTimer);
    const sceneId = table.scene?.id;
    const apply = () => {
      if (!sceneId || table.scene?.id !== sceneId || table.role !== 'gm' || table.viewAs) return;
      send({ type: 'setEnvironment', environment });
      editor.syncLiveEnvironment(table.scene.id, environment);
    };
    if (commit) apply();
    else lightingTimer = setTimeout(apply, 100);
  }

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

  const tabs: { id: TableMode; name: string }[] = [
    { id: 'play', name: 'Play' }, { id: 'map', name: 'Map' },
    { id: 'campaign', name: 'Campaign' }, { id: 'rules', name: 'Rules' },
  ];
  function selectMode(next: TableMode) {
    closeMenus();
    modes.set(next);
    layout.focusMode = false;
    layout.leftOpen = next !== 'play';
    if (window.innerWidth < 1000) layout.rightOpen = false;
  }
  function shortcuts(event: KeyboardEvent) {
    if (event.defaultPrevented) return;
    if ((event.target as HTMLElement)?.closest('input, textarea, select, [contenteditable=true]') || event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key === 'Escape' && table.menu) return;
    if (event.key.toLowerCase() === 'f') layout.focusMode = !layout.focusMode;
    if (event.key === 'Escape') layout.focusMode = false;
  }

  let seatMenu = $state<{ id: string; name: string; anchor: Anchor } | null>(null);

  function dismiss(playerId: string) {
    const player = table.players.find((item) => item.id === playerId);
    if (!gm || !player || player.id === table.playerId || player.connected) return;
    send({ type: 'removePlayer', playerId });
  }

  function seatTitle(player: PlayerInfo): string {
    const role = player.role === 'gm' ? ' · GM' : '';
    if (!gm || player.id === table.playerId) return `${player.name}${role}`;
    const gone = player.connected ? '' : ' · click to dismiss';
    return `${player.name}${role}${gone} · right-click to kick`;
  }

  function openSeat(event: MouseEvent, player: PlayerInfo) {
    event.preventDefault();
    if (!gm || player.id === table.playerId) {
      seatMenu = null;
      return;
    }
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    seatTrigger = event.currentTarget as HTMLElement;
    seatMenu = {
      id: player.id,
      name: player.name,
      anchor: { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom },
    };
  }

  function openPanel(id: 'talk' | 'settings') {
    closeMenus();
    const open = id === 'talk' ? chrome.talk : chrome.settings;
    if (id === 'talk') chrome.talk = !open;
    else chrome.settings = !open;
    if (open) return;
    const place = layout.place(id);
    layout.patch(id, {
      floating: true,
      collapsed: false,
      x: id === 'talk' ? 24 : Math.max(24, window.innerWidth - 460),
      y: 72,
      w: Math.max(place.w, id === 'talk' ? 360 : 420),
      h: Math.max(place.h, 480),
    });
  }

  function kick(playerId: string) {
    seatMenu = null;
    send({ type: 'removePlayer', playerId });
  }

  function resetWorkspace() {
    closeMenus(null, true);
    chrome.settings = false;
    chrome.talk = false;
    systems.sheetOpen = false;
    layout.reset();
  }

  $effect(() => {
    if (!seatMenu) return;
    const openId = seatMenu.id;
    if (!gm || !table.players.some((player) => player.id === openId)) seatMenu = null;
  });
</script>

<svelte:window onkeydown={shortcuts} />

{#snippet popoverHeading(eyebrow: string, title: string, emblem: string)}
  <div class="popover-heading">
    <span class="popover-emblem" aria-hidden="true">{emblem}</span>
    <div class="popover-heading-text">
      <p class="popover-eyebrow">{eyebrow}</p>
      <h2 class="popover-title">{title}</h2>
    </div>
    <button class="popover-close" type="button" aria-label={`Close ${title}`} title="Close (Esc)" onclick={() => closeMenus(null, true)}>×</button>
  </div>
{/snippet}

<header bind:this={headerEl} class="topbar" data-mode={mode}>
  <div class="table-identity" title={`${table.campaign.name || 'Campaign'} · ${systemName} · ${expectation}`}>
    <span class="brand-mark" aria-hidden="true">L</span>
    <div class="min-w-0">
      <p class="display brand-name">LONGTABLE <span class="mode-indicator">{mode}</span></p>
      <p class="scene-name">{sceneName}</p>
    </div>
  </div>
  {#if gm}
    <nav class="mode-nav" aria-label="Table mode">
      {#each tabs as tab (tab.id)}
        <button class:active={modes.current === tab.id} aria-current={modes.current === tab.id ? 'page' : undefined} type="button" onclick={() => selectMode(tab.id)}>{tab.name}</button>
      {/each}
    </nav>
  {/if}
  <div class="topbar-actions">
    <details class="topbar-menu" ontoggle={menuToggled}>
      <summary class="btn quiet">Sounds <span aria-hidden="true">⌄</span></summary>
      <div class="topbar-menu-body">
        {@render popoverHeading('Around the table', 'Sound & voices', '♫')}
        <div class="popover-body"><Soundboard /></div>
      </div>
    </details>
    {#if gm && table.scene && mode === 'play'}
      <details class="topbar-menu" ontoggle={menuToggled}>
        <summary class="btn quiet">Lighting <span aria-hidden="true">⌄</span></summary>
        <div class="topbar-menu-body">
          {@render popoverHeading('Set the atmosphere', 'Light & time', '☼')}
          <div class="popover-body">
            <EnvironmentControls environment={table.scene.environment} onchange={changeEnvironment} />
            <p class="shortcut-hint mt-3">Applies to everyone at the table. Add sources in Map → Lights.</p>
          </div>
        </div>
      </details>
    {/if}
    <details class="topbar-menu view-menu" ontoggle={menuToggled}>
      <summary class="btn quiet">View <span aria-hidden="true">⌄</span></summary>
      <div class="topbar-menu-body">
        {@render popoverHeading('Survey the scene', 'View of the table', '⌖')}
        <div class="popover-body">
          <ViewControls {onZoom} {onFit} {onMe} {onCycle} {onSetGrid} />
          <p class="shortcut-hint">Wheel to zoom · middle-drag to pan</p>
          <p class="shortcut-hint"><kbd>0</kbd> fit · <kbd>C</kbd> your figure · <kbd>G</kbd> grid</p>
        </div>
      </div>
    </details>
    <button class="btn quiet focus-control" class:active={layout.focusMode} aria-pressed={layout.focusMode} title="Focus on the scene (F)" type="button" onclick={() => layout.focusMode = !layout.focusMode}>{layout.focusMode ? 'Show tools' : 'Focus'}</button>
    <button class="btn quiet" class:active={chrome.talk} type="button" onclick={() => openPanel('talk')}>Talk</button>
    <button class="btn quiet" class:active={chrome.settings} type="button" onclick={() => openPanel('settings')}>Settings{account.lobby && account.lobby.requests.length > 0 ? ` · ${account.lobby.requests.length}` : ''}</button>
    {#if table.role === 'gm' && table.viewAs}
      <button class="btn" type="button" onclick={() => send({ type: 'viewAs', playerId: null })}>Your eyes</button>
    {/if}
    <details class="topbar-menu" ontoggle={menuToggled}>
      <summary class="btn quiet" aria-label="Table menu">•••</summary>
      <div class="topbar-menu-body">
        {@render popoverHeading(systemName, table.campaign.name || 'Your table', '◇')}
        <div class="popover-body table-options">
        <p class="popover-section-label">Seats at the table</p>
        <div class="flex flex-wrap items-center gap-2 py-2" aria-label="Players">
          {#each table.players as player (player.id)}
            <button class="portrait portrait-sm" style={`--ring:${player.color}`} type="button" title={seatTitle(player)} aria-label={seatTitle(player)} onclick={() => dismiss(player.id)} oncontextmenu={(event) => openSeat(event, player)}>
              {player.name.slice(0, 2).toUpperCase()}
              <span class={`seat-dot ${player.connected ? 'bg-emerald-400' : 'bg-stone-500'}`}></span>
            </button>
          {/each}
        </div>
        <button class="btn quiet" type="button" onclick={resetWorkspace}>Reset layout</button>
        <button class="btn quiet" type="button" onclick={() => { closeMenus(); leaveTable(); }}>Leave the table</button>
        <span class="shortcut-hint" title={VERSION_NOTE}>Longtable v{VERSION}</span>
        </div>
      </div>
    </details>
    {#if seatMenu}
      <SeatMenu name={seatMenu.name} anchor={seatMenu.anchor} onkick={() => seatMenu && kick(seatMenu.id)} onclose={() => closeSeat(true)} />
    {/if}
  </div>
</header>

<style>
  .topbar:has(details[open]) { z-index: var(--z-menu); }
  @media (max-width: 760px) {
    .topbar-actions { flex-wrap: wrap; justify-content: flex-start; }
    .topbar-menu { position: static; }
    .topbar-menu .topbar-menu-body {
      left: 8px;
      right: 8px;
      top: calc(100% + 6px);
      width: auto;
      min-width: 0;
      max-width: none;
      max-height: calc(100dvh - 180px);
      overflow-y: auto;
    }
  }
</style>
