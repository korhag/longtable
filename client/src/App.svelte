<script lang="ts">
  import { onMount } from 'svelte';
  import { ensureCatalog, findIn, loadCatalog } from './lib/assets/catalog';
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
  import { account } from './lib/account.svelte';
  import { chrome } from './lib/chrome.svelte';
  import { PROFILE_ID } from './lib/render/cameraProfile';
  import { TableEngine } from './lib/render/engine';
  import { castFor } from './lib/render/rings';
  import { cellLevel, tierLabel } from './lib/render/levels';
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
  import SettingsPanel from './lib/ui/SettingsPanel.svelte';
  import TalkPanel from './lib/ui/TalkPanel.svelte';
  import TokenPanel from './lib/ui/TokenPanel.svelte';
  import ContextDial from './lib/ui/ContextDial.svelte';

  let host: HTMLDivElement;
  let viewportWidth = $state(1400);
  let topHeight = $state(60);
  let bottomHeight = $state(0);
  let topBar = $state<HTMLElement | undefined>();
  let bottomStrip = $state<HTMLElement | undefined>();
  let engine: TableEngine | undefined;
  let ready = $state(false);
  let autoSelectedSeat = $state<string | null>(null);
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
          kind: light.kind,
          intensity: light.intensity,
        })),
        layout: editor.draft.layout,
        environment: editor.draft.layout.environment,
        presentation: null,
        schemaVersion: 1,
        visualProfile: PROFILE_ID,
      }
    : table.scene);

  const figureColors = ['#d4a656', '#c45c4a', '#6e9a72', '#6f8fbf', '#8d5b86', '#e7d7b6'];

  function spanFor(asset: string | null | undefined): [number, number] {
    if (!asset) return [1, 1];
    const found = findIn(editor.packs, asset, (pack) => pack.cast ?? []);
    return found?.def.suggestedFootprintCells ?? [1, 1];
  }

  function armedSpan(): [number, number] | null {
    if (editor.active || table.viewAs || table.role !== 'gm') return null;
    if (table.placingId) {
      const character = table.characters.find((item) => item.id === table.placingId);
      return spanFor(character?.castAssetId);
    }
    if (table.placingToken) return spanFor(editor.castId || null);
    return null;
  }

  function placeFigure(point: { x: number; y: number }) {
    const index = table.tokens.length;
    const color = figureColors[index % figureColors.length] ?? '#d4a656';
    if (table.placingId) {
      const character = table.characters.find((item) => item.id === table.placingId);
      table.placingId = null;
      if (!character) return;
      send({
        type: 'createToken',
        name: character.name,
        x: point.x,
        y: point.y,
        ownerId: character.ownerId,
        hidden: character.kind !== 'pc',
        visionRadius: character.kind === 'pc' ? 12 : 8,
        grantsVision: character.kind === 'pc',
        color: character.kind === 'adversary' ? '#c45c4a' : color,
        characterId: character.id,
        castAsset: null,
        facing: 'south',
      });
      table.tool = 'select';
      return;
    }
    if (!table.placingToken) return;
    table.placingToken = false;
    send({
      type: 'createToken',
      name: table.placingName.trim() || 'Token',
      x: point.x,
      y: point.y,
      ownerId: null,
      hidden: false,
      visionRadius: 12,
      grantsVision: true,
      color,
      characterId: null,
      castAsset: editor.castId || null,
      facing: 'south',
    });
  }

  onMount(() => {
    engine = new TableEngine(host, {
      onMove: (tokenId, x, y) => send({ type: 'moveToken', tokenId, x, y }),
      onSelect: (tokenId) => {
        table.selectedTokenId = tokenId;
        table.targetTokenId = null;
        table.menu = null;
        const token = table.tokens.find((item) => item.id === tokenId);
        if (token?.characterId) systems.selectedId = token.characterId;
      },
      onTarget: (tokenId) => {
        table.targetTokenId = tokenId;
        table.menu = null;
      },
      onMenu: (tokenId, x, y, point) => { table.menu = { tokenId, x, y, point }; },
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
      onPlace: (point) => placeFigure(point),
      onCancelPlace: () => {
        table.placingId = null;
        table.placingToken = false;
      },
      canDrag: (token) => !table.viewAs && (table.role === 'gm' || token.ownerId === table.playerId),
      onMeasure: (delta) => {
        if (!delta) {
          measureLabel = null;
          return;
        }
        const ground = tierLabel(cellLevel(table.scene?.layout, delta.landX, delta.landY));
        if (delta.dx === 0 && delta.dy === 0) {
          measureLabel = ground;
        } else {
          const result = measure(delta.dx, delta.dy, systems.active?.grid ?? null);
          measureLabel = result.range ? `${result.text} · ${result.range} · ${ground}` : `${result.text} · ${ground}`;
        }
        if (delta.note) measureLabel = `${measureLabel}. ${delta.note}`;
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
      seatId: table.viewAs ?? table.playerId,
      selectedId: table.selectedTokenId,
      targetId: table.targetTokenId,
      hoverId: table.hoverTokenId,
      tool: editor.active || table.viewAs ? 'select' : table.tool,
      packs: editor.packs,
      ghost: editor.ghost,
      placeSpan: armedSpan(),
      buildMode: editor.active,
      viewId: editor.active ? '' : table.viewId,
      cast: Object.fromEntries(table.tokens.map((token) => [token.id, castFor(token, table.characters, table.players)])),
    });
  });

  $effect(() => {
    if (!table.joined || !account.token) return;
    const timer = window.setInterval(() => {
      void account.refresh();
    }, 8000);
    return () => window.clearInterval(timer);
  });

  $effect(() => {
    if (!table.joined) return;
    const revision = table.catalogRevision;
    let cancelled = false;
    void ensureCatalog(revision)
      .then((packs) => {
        if (!cancelled && table.catalogRevision === revision) editor.packs = packs;
      })
      .catch(() => {
        if (!cancelled) table.error = 'The asset library did not load. It will try again.';
      });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    if (!table.error || !table.joined) return;
    const timer = window.setTimeout(() => {
      table.error = null;
    }, 4200);
    return () => window.clearTimeout(timer);
  });

  $effect(() => {
    const elements = [topBar, bottomStrip].filter((element): element is HTMLElement => !!element);
    const update = () => {
      topHeight = topBar?.offsetHeight ?? 60;
      bottomHeight = bottomStrip?.offsetHeight ?? 0;
    };
    const observer = new ResizeObserver(update);
    elements.forEach((element) => observer.observe(element));
    update();
    return () => observer.disconnect();
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
      left: layout.focusMode || !gmTools ? 0 : layout.leftOpen && viewportWidth >= 760 ? Math.min(layout.leftWidth, viewportWidth - 104) : 44,
      right: layout.focusMode ? 0 : layout.rightOpen && viewportWidth >= 760 ? Math.min(layout.rightWidth, viewportWidth - 104) : 44,
      top: topHeight,
      bottom: layout.focusMode ? 0 : bottomHeight,
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
    if (table.role !== 'player' || table.viewAs || table.selectedTokenId || autoSelectedSeat === table.playerId) return;
    const owned = table.tokens.filter((token) => token.ownerId === table.playerId);
    if (owned.length !== 1) return;
    const token = owned[0];
    if (!token) return;
    autoSelectedSeat = table.playerId;
    table.selectedTokenId = token.id;
    if (token.characterId) systems.selectedId = token.characterId;
  });

  $effect(() => {
    if (viewportWidth < 1000 && layout.leftOpen && layout.rightOpen) layout.rightOpen = false;
  });

  function mine() {
    const characterId = systems.chosen?.id;
    return table.tokens.find((token) => characterId && token.characterId === characterId)
      ?? table.tokens.find((token) => token.ownerId === table.playerId)
      ?? null;
  }
</script>

<svelte:window bind:innerWidth={viewportWidth} />

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
        {#if table.role === 'gm' && !table.viewAs && !layout.focusMode}
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
          {#if PlayChrome && modes.current === 'play' && !editor.active && !layout.focusMode}
            <PlayChrome {aimPoint} onCenter={(x, y) => engine?.centerOn(x, y)} />
          {/if}
          {#if modes.current === 'rules' && table.role === 'gm' && !table.viewAs && !layout.focusMode}
            <div class="pointer-events-none absolute inset-0 z-10 flex p-3">
              <RulesBoard />
            </div>
          {/if}
          {#if !layout.focusMode && !(modes.current === 'rules' && table.role === 'gm' && !table.viewAs)}
          <div bind:this={bottomStrip} class="scene-command-strip">
            {#if table.selectedTokenId}
              <div class="selection-slot"><TokenPanel /></div>
            {/if}
            {#if !editor.active && systems.active && (systems.chosen || table.role === 'gm' && !table.viewAs)}
              <div class="character-slot"><CharacterBar /></div>
            {/if}
          </div>
          {/if}
        </div>
        {#if !layout.focusMode}
        <Dock side="right">
          <Panel id="rolls" title="Rolls" grow><RollLog /></Panel>
          <Panel id="dice" title="Dice"><DiceBar /></Panel>
        </Dock>
        {/if}
      </div>
    </div>

    <DiceStage />
    {#if !editor.active}
      <ContextDial onCenter={(x, y) => engine?.centerOn(x, y)} onMove={(id, x, y) => engine?.moveTo(id, x, y)} onTeleport={(tokenId, x, y) => { engine?.skipNextMovement(tokenId); send({ type: 'teleportToken', tokenId, x, y }); }} />
    {/if}

    {#if chrome.settings}
      <Panel id="settings" title="Settings" onclose={() => (chrome.settings = false)}>
        <SettingsPanel />
      </Panel>
    {/if}
    {#if chrome.talk}
      <Panel id="talk" title="Talk" onclose={() => (chrome.talk = false)}>
        <TalkPanel />
      </Panel>
    {/if}

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
    {#if table.review}
      <div class="glass fixed top-16 left-1/2 z-[var(--z-float)] w-[min(36rem,calc(100%-2rem))] -translate-x-1/2 rounded-lg px-4 py-3 text-sm">
        <p>{table.review}</p>
        <button class="btn quiet mt-2 text-sm" type="button" onclick={() => table.dismissReview()}>Dismiss</button>
      </div>
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
