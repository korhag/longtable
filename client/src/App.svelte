<script lang="ts">
  import { onMount } from 'svelte';
  import { loadCatalog } from './lib/assets/catalog';
  import CampaignPanel from './lib/campaign/CampaignPanel.svelte';
  import RulesPanel from './lib/designer/RulesPanel.svelte';
  import RulesBoard from './lib/designer/RulesBoard.svelte';
  import BuildMode from './lib/editor/BuildMode.svelte';
  import { editor } from './lib/editor/draft.svelte';
  import Dock from './lib/layout/Dock.svelte';
  import { layout } from './lib/layout/layout.svelte';
  import { modes } from './lib/layout/modes.svelte';
  import Panel from './lib/layout/Panel.svelte';
  import TopBar from './lib/layout/TopBar.svelte';
  import type { SceneView } from './lib/net/bindings/SceneView.ts';
  import { send } from './lib/net/socket';
  import { TableEngine } from './lib/render/engine';
  import { measure } from './lib/systems/measure';
  import { systems } from './lib/systems/store.svelte';
  import { overlaysFor, playChromeFor } from './systems';
  import { table } from './lib/table.svelte';
  import CharacterBar from './lib/ui/CharacterBar.svelte';
  import DiceBar from './lib/ui/DiceBar.svelte';
  import DiceStage from './lib/ui/DiceStage.svelte';
  import GmToolbar from './lib/ui/GmToolbar.svelte';
  import Lobby from './lib/ui/Lobby.svelte';
  import RollLog from './lib/ui/RollLog.svelte';
  import TokenPanel from './lib/ui/TokenPanel.svelte';

  let host: HTMLDivElement;
  let topBar = $state<HTMLElement | undefined>();
  let bottomStrip = $state<HTMLElement | undefined>();
  let engine: TableEngine | undefined;
  let ready = $state(false);
  let measureLabel = $state<string | null>(null);
  let measureAt = $state({ x: 0, y: 0 });
  let aimPoint = $state<{ x: number; y: number } | null>(null);

  const shown = $derived<SceneView | null>(editor.active && editor.draft
    ? {
        id: editor.draft.id,
        name: editor.draft.name,
        width: editor.draft.layout.width,
        height: editor.draft.layout.height,
        imageUrl: null,
        walls: [],
        lights: (editor.draft.layout.lights ?? []).map((light) => ({
          x: light.x,
          y: light.y,
          range: light.range,
          color: light.color,
        })),
        layout: editor.draft.layout,
        presentation: null,
      }
    : table.scene);

  onMount(() => {
    engine = new TableEngine(host, {
      onMove: (tokenId, x, y) => send({ type: 'moveToken', tokenId, x, y }),
      onSelect: (tokenId) => {
        table.selectedTokenId = tokenId;
        table.menu = null;
        const token = table.tokens.find((item) => item.id === tokenId);
        if (token?.characterId) systems.selectedId = token.characterId;
      },
      onTarget: (tokenId) => {
        table.targetTokenId = tokenId;
        table.menu = null;
      },
      onMenu: (tokenId, x, y) => {
        if (!tokenId) {
          table.menu = null;
          return;
        }
        table.menu = { tokenId, x, y };
      },
      onPaint: () => {
        const id = table.targetTokenId;
        const raw = id ? engine?.screenOf(id) ?? null : null;
        const next = raw ? { x: Math.round(raw.x), y: Math.round(raw.y) } : null;
        if (next?.x === aimPoint?.x && next?.y === aimPoint?.y) return;
        if (!next && !aimPoint) return;
        aimPoint = next;
      },
      onBrush: (mode, cells) => send({ type: 'fogBrush', mode, cells }),
      onToggleDoor: (doorId, open) => send({ type: 'setDoor', doorId, open }),
      onBuild: (phase, point, button) => editor.pointer(phase, point, button),
      canDrag: (token) => !table.viewAs && (table.role === 'gm' || token.ownerId === table.playerId),
      onMeasure: (delta) => {
        if (!delta || (delta.dx === 0 && delta.dy === 0)) {
          measureLabel = null;
          return;
        }
        const result = measure(delta.dx, delta.dy, systems.active?.grid ?? null);
        measureLabel = result.range ? `${result.text} · ${result.range}` : result.text;
        measureAt = { x: delta.x, y: delta.y };
      },
      onWarning: (message) => {
        table.error = message;
      },
    });
    engine.start();
    ready = true;
    return () => {
      ready = false;
      engine?.dispose();
    };
  });

  $effect(() => {
    if (!ready || !engine) return;
    engine.sync({
      scene: shown,
      tokens: table.tokens,
      doors: editor.active ? [] : table.doors,
      fog: editor.active ? null : table.fog,
      role: table.viewAs ? 'player' : table.role,
      selectedId: table.selectedTokenId,
      targetId: table.targetTokenId,
      hoverId: table.hoverTokenId,
      tool: editor.active || table.viewAs ? 'select' : table.tool,
      packs: editor.packs,
      ghost: editor.ghost,
      buildMode: editor.active,
      viewId: editor.active ? '' : table.viewId,
    });
  });

  $effect(() => {
    if (!table.joined || editor.packs.length > 0) return;
    void loadCatalog().then((packs) => {
      if (editor.packs.length === 0) editor.packs = packs;
    });
  });

  $effect(() => {
    if (!table.error || !table.joined) return;
    const timer = window.setTimeout(() => {
      table.error = null;
    }, 4200);
    return () => window.clearTimeout(timer);
  });

  $effect(() => {
    if (!ready || !engine) return;
    layout.leftWidth;
    layout.rightWidth;
    modes.current;
    table.role;
    table.viewAs;
    table.joined;
    const gmTools = table.role === 'gm' && !table.viewAs;
    engine.setInsets({
      left: gmTools ? layout.leftWidth : 0,
      right: layout.rightWidth,
      top: topBar?.offsetHeight ?? 52,
      bottom: bottomStrip?.offsetHeight ?? 0,
    });
  });

  $effect(() => {
    if (!table.concealedNotice) return;
    const timer = window.setTimeout(() => table.clearNotice(), 4200);
    return () => window.clearTimeout(timer);
  });
  const Overlays = $derived(overlaysFor(table.campaign.systemId));
  const PlayChrome = $derived(playChromeFor(table.campaign.systemId));

  $effect(() => {
    if (table.role !== 'player' || table.viewAs || table.selectedTokenId) return;
    const owned = table.tokens.filter((token) => token.ownerId === table.playerId);
    if (owned.length !== 1) return;
    const token = owned[0];
    if (!token) return;
    table.selectedTokenId = token.id;
    if (token.characterId) systems.selectedId = token.characterId;
  });

  function mine() {
    const characterId = systems.chosen?.id;
    return table.tokens.find((token) => characterId && token.characterId === characterId)
      ?? table.tokens.find((token) => token.ownerId === table.playerId)
      ?? null;
  }
</script>

<div class="relative h-full w-full">
  <div class="absolute inset-0" bind:this={host}></div>
  <div class="vignette"></div>

  {#if table.joined}
    <div class="pointer-events-none absolute inset-0 flex flex-col">
      <div bind:this={topBar} class="pointer-events-auto">
        <TopBar
          onZoom={(factor) => engine?.zoomBy(factor)}
          onFit={() => engine?.fitMap()}
          onMe={() => { const token = mine(); if (token) engine?.centerOn(token.x, token.y); }}
          onCycle={() => engine?.cycleGrid() ?? 'faint'}
          onSetGrid={(style) => engine?.setGrid(style)}
        />
      </div>
      <div class="flex min-h-0 flex-1">
        {#if table.role === 'gm' && !table.viewAs}
          <Dock side="left">
            {#if modes.current === 'play'}
              <Panel id="play" title="Play" grow><GmToolbar /></Panel>
            {:else if modes.current === 'map'}
              <Panel id="map" title="Map" grow><BuildMode /></Panel>
            {:else if modes.current === 'campaign'}
              <Panel id="campaign" title="Campaign" grow><CampaignPanel /></Panel>
            {:else}
              <Panel id="rules" title="Rules" grow><RulesPanel /></Panel>
            {/if}
          </Dock>
        {/if}
        <div class="relative min-w-0 flex-1">
          {#if PlayChrome && modes.current === 'play' && !editor.active}
            <PlayChrome {aimPoint} onCenter={(x, y) => engine?.centerOn(x, y)} />
          {/if}
          {#if modes.current === 'rules' && table.role === 'gm' && !table.viewAs}
            <div class="pointer-events-none absolute inset-0 z-10 flex p-3">
              <RulesBoard />
            </div>
          {/if}
          {#if !(modes.current === 'rules' && table.role === 'gm' && !table.viewAs)}
          <div bind:this={bottomStrip} class="pointer-events-none absolute inset-x-0 bottom-0 flex items-end gap-2 p-2">
            {#if table.selectedTokenId}
              <div class="pointer-events-auto w-[min(22rem,40%)]">
                <Panel id="token" title="Token"><TokenPanel /></Panel>
              </div>
            {/if}
            <div class="pointer-events-auto min-w-0 flex-1">
              <Panel id="character" title="Character"><CharacterBar /></Panel>
            </div>
          </div>
          {/if}
        </div>
        <Dock side="right">
          <Panel id="rolls" title="Rolls" grow><RollLog /></Panel>
          <Panel id="dice" title="Dice"><DiceBar /></Panel>
        </Dock>
      </div>
    </div>

    <DiceStage />

    {#if Overlays}
      <Overlays />
    {/if}

    {#if measureLabel}
      <p class="pointer-events-none fixed z-[var(--z-float)] rounded-full bg-[#1c140e]/90 px-2 py-1 text-sm text-[var(--color-gold-bright)]" style={`left:${measureAt.x + 14}px;top:${measureAt.y + 14}px`}>
        {measureLabel}
      </p>
    {/if}

    {#if table.concealedNotice}
      <p class="glass toast pointer-events-none fixed top-16 left-1/2 -translate-x-1/2 rounded-full px-4 py-2 text-sm">
        {table.concealedNotice}
      </p>
    {/if}
    {#if table.error}
      <p class="glass toast pointer-events-none fixed bottom-24 left-1/2 -translate-x-1/2 rounded-full px-4 py-2 text-sm text-[#e7a090]">
        {table.error}
      </p>
    {/if}
  {:else}
    <Lobby />
  {/if}
</div>
