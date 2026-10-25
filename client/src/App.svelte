<script lang="ts">
  import { onMount } from 'svelte';
  import { loadCatalog } from './lib/assets/catalog';
  import BuildMode from './lib/editor/BuildMode.svelte';
  import { editor } from './lib/editor/draft.svelte';
  import { designer } from './lib/designer/designer.svelte';
  import SystemDesigner from './lib/designer/SystemDesigner.svelte';
  import { leaveTable, send } from './lib/net/socket';
  import { TableEngine } from './lib/render/engine';
  import { measure } from './lib/systems/measure';
  import { systems } from './lib/systems/store.svelte';
  import { table } from './lib/table.svelte';
  import CharacterBar from './lib/ui/CharacterBar.svelte';
  import DiceDock from './lib/ui/DiceDock.svelte';
  import GmToolbar from './lib/ui/GmToolbar.svelte';
  import Lobby from './lib/ui/Lobby.svelte';
  import Party from './lib/ui/Party.svelte';
  import TokenPanel from './lib/ui/TokenPanel.svelte';
  import ViewControls from './lib/ui/ViewControls.svelte';
  import type { SceneView } from './lib/net/bindings/SceneView.ts';

  let host: HTMLDivElement;
  let engine: TableEngine | undefined;
  let ready = $state(false);
  let measureLabel = $state<string | null>(null);
  let measureAt = $state({ x: 0, y: 0 });

  const shown = $derived<SceneView | null>(editor.active && editor.draft
    ? {
        id: editor.draft.id,
        name: editor.draft.name,
        width: editor.draft.layout.width,
        height: editor.draft.layout.height,
        imageUrl: null,
        walls: [],
        lights: [],
        layout: editor.draft.layout,
      }
    : table.scene);

  onMount(() => {
    engine = new TableEngine(host, {
      onMove: (tokenId, x, y) => send({ type: 'moveToken', tokenId, x, y }),
      onSelect: (tokenId) => {
        table.selectedTokenId = tokenId;
        const token = table.tokens.find((item) => item.id === tokenId);
        if (token?.characterId) systems.selectedId = token.characterId;
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
      tool: editor.active || table.viewAs ? 'select' : table.tool,
      packs: editor.packs,
      ghost: editor.ghost,
      buildMode: editor.active,
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
    if (!table.concealedNotice) return;
    const timer = window.setTimeout(() => table.clearNotice(), 4200);
    return () => window.clearTimeout(timer);
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
    <Party />
    {#if table.role === 'gm' && designer.active}
      <SystemDesigner />
    {:else if table.role === 'gm' && editor.active}
      <BuildMode />
    {:else if table.role === 'gm'}
      <GmToolbar />
    {/if}
    <div class="pointer-events-none fixed bottom-4 left-4 z-10 flex w-[min(42rem,calc(100vw-28rem))] flex-col items-stretch gap-2">
      <TokenPanel />
      {#if !designer.active}
        <CharacterBar />
      {/if}
    </div>
    <DiceDock />

    <div class="glass pointer-events-auto fixed top-4 left-1/2 z-10 flex -translate-x-1/2 items-center gap-3 rounded-full px-4 py-2 text-xs">
      <span class="tracking-[0.18em] text-[var(--color-gold)] uppercase">{table.connected ? 'At the table' : 'Reconnecting'}</span>
      <ViewControls
        onZoom={(factor) => engine?.zoomBy(factor)}
        onFit={() => engine?.fitMap()}
        onMe={() => { const token = mine(); if (token) engine?.centerOn(token.x, token.y); }}
        onCycle={() => engine?.cycleGrid() ?? 'faint'}
        onSetGrid={(style) => engine?.setGrid(style)}
      />
      {#if table.role === 'gm'}
        <button class="btn quiet px-2 py-1" type="button" onclick={() => { designer.active = false; editor.leave(); }}>Play</button>
        <button class="btn quiet px-2 py-1" type="button" onclick={() => { designer.active = false; void editor.enter(); }}>Build</button>
        <button class="btn quiet px-2 py-1" type="button" onclick={() => { editor.leave(); designer.active = true; }}>Rules</button>
      {/if}
      <button class="btn quiet px-2 py-1" type="button" onclick={() => leaveTable()}>Leave</button>
    </div>

    {#if measureLabel}
      <p class="pointer-events-none fixed z-30 rounded-full bg-[#1c140e]/90 px-2 py-1 text-xs text-[var(--color-gold-bright)]" style={`left:${measureAt.x + 14}px;top:${measureAt.y + 14}px`}>
        {measureLabel}
      </p>
    {/if}

    {#if table.concealedNotice}
      <p class="glass pointer-events-none fixed top-16 left-1/2 z-20 -translate-x-1/2 rounded-full px-4 py-2 text-sm">
        {table.concealedNotice}
      </p>
    {/if}
    {#if table.error}
      <p class="glass pointer-events-none fixed bottom-24 left-1/2 z-20 -translate-x-1/2 rounded-full px-4 py-2 text-sm text-[#e7a090]">
        {table.error}
      </p>
    {/if}
  {:else}
    <Lobby />
  {/if}
</div>
