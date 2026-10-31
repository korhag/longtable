<script lang="ts">
  import { onMount, tick, untrack, type Snippet } from 'svelte';
  import { layout, type PanelPlace } from './layout.svelte';
  import { panelBounds } from './panelBounds';

  let { id, title, children, onclose, grow = false, dockable = true }: { id: string; title: string; children: Snippet; onclose?: () => void; grow?: boolean; dockable?: boolean } = $props();

  let viewportWidth = $state(1400);
  let viewportHeight = $state(900);
  let panelElement: HTMLElement;
  let titleElement: HTMLElement;
  let opener: HTMLElement | null = null;
  const place = $derived(layout.place(id));
  const floating = $derived(place.floating);
  const closable = $derived(!!onclose);
  const box = $derived(panelBounds(place, viewportWidth, viewportHeight));
  const headingId = $derived(`panel-${id}-title`);
  const instructionsId = $derived(`panel-${id}-movement`);
  const caption = $derived(({
    sheet: 'Character record', settings: 'Table preferences', talk: 'Around the table',
    play: 'Game master', map: 'Mapmaker’s tools', campaign: 'Campaign journal',
    rules: 'Rules reference', rolls: 'Roll chronicle', dice: 'Dice & fate',
  } as Record<string, string>)[id]);
  const emblem = $derived(({
    sheet: 'M4 4h6a3 3 0 0 1 3 3v14a4 4 0 0 0-4-2H4z M20 4h-4a3 3 0 0 0-3 3 M20 4v15h-3a4 4 0 0 0-4 2',
    settings: 'M4 6h16 M4 12h16 M4 18h16 M8 3v6 M16 9v6 M10 15v6',
    talk: 'M4 4h16v12H9l-5 4z M8 8h8 M8 12h5',
    map: 'M3 5l6-2 6 2 6-2v16l-6 2-6-2-6 2z M9 3v16 M15 5v16',
    campaign: 'M5 3h14v18l-7-4-7 4z M9 7h6 M9 11h6',
    rules: 'M5 3h14v18H5z M8 7h8 M8 11h8 M8 15h5',
    rolls: 'M6 3h12v16H6z M6 3H4v4h2 M18 19h2v-4h-2 M9 7h6 M9 11h6 M9 15h4',
  } as Record<string, string>)[id] ?? 'M12 2l9 6v8l-9 6-9-6V8z M3 8h18 M3 16l9-14 9 14H3 M12 22V8');

  $effect(() => {
    const key = id, isFloating = floating, canClose = closable;
    untrack(() => layout.registerWindow(key, isFloating, canClose));
    return () => untrack(() => layout.removeWindow(key));
  });

  function raise() { if (place.floating) layout.raiseWindow(id); }

  // A floating window must leave dock overflow and stacking contexts. Keep a
  // marker so docking returns it to the same Svelte-owned content position.
  function floatWindow(node: HTMLElement, isFloating: boolean) {
    const marker = document.createComment('panel-position');
    node.before(marker);
    const move = (value: boolean) => {
      if (value) document.body.append(node);
      else marker.parentNode?.insertBefore(node, marker.nextSibling);
    };
    move(isFloating);
    return {
      update: move,
      destroy() { marker.remove(); node.remove(); },
    };
  }

  function close() {
    const returnFocus = panelElement?.contains(document.activeElement);
    onclose?.();
    if (returnFocus) void tick().then(() => { if (opener?.isConnected) opener.focus({ preventScroll: true }); });
  }

  function escape(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented || event.isComposing || !place.floating || !onclose) return;
    if (layout.frontmostClosableWindow !== id) return;
    if (event.target instanceof Element && event.target.closest('input, textarea, select, [contenteditable=true]')) return;
    // Higher layers consume Escape first. Some use window listeners, so their
    // visible presence also guards against closing a window behind the menu.
    if (document.querySelector('.context-dial, .topbar details[open], .menu-shade, .dh-overlay, .dice-overlay:not(.away), dialog[open], [aria-modal=true]')) return;
    event.preventDefault();
    close();
  }

  onMount(() => {
    opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    panelElement.addEventListener('pointerdown', raise, true);
    panelElement.addEventListener('focusin', raise);
    document.addEventListener('keydown', escape);
    if (place.floating) void tick().then(() => titleElement?.focus({ preventScroll: true }));
    return () => {
      panelElement.removeEventListener('pointerdown', raise, true);
      panelElement.removeEventListener('focusin', raise);
      document.removeEventListener('keydown', escape);
    };
  });

  function moveKeys(event: KeyboardEvent) {
    if (!place.floating || event.target !== event.currentTarget || !event.key.startsWith('Arrow')) return;
    event.preventDefault();
    const step = event.shiftKey ? 40 : 10;
    const dx = event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0;
    const dy = event.key === 'ArrowDown' ? step : event.key === 'ArrowUp' ? -step : 0;
    layout.patch(id, moved({ ...place, ...box }, dx, dy));
  }

  function resizeKeys(event: KeyboardEvent) {
    if (!event.key.startsWith('Arrow')) return;
    event.preventDefault();
    const delta = event.shiftKey ? 40 : 10;
    layout.patch(id, panelBounds({ ...box,
      w: box.w + (event.key === 'ArrowRight' ? delta : event.key === 'ArrowLeft' ? -delta : 0),
      h: box.h + (event.key === 'ArrowDown' ? delta : event.key === 'ArrowUp' ? -delta : 0),
    }, viewportWidth, viewportHeight));
  }

  function drag(event: PointerEvent) {
    if (!place.floating || event.button !== 0) return;
    if ((event.target as HTMLElement).closest('button')) return;
    const startX = event.clientX;
    const startY = event.clientY;
    const origin = { ...place, ...box };
    const move = (next: PointerEvent) => {
      const box = moved(origin, next.clientX - startX, next.clientY - startY);
      layout.patch(id, box);
    };
    track(event, move);
  }

  function resize(event: PointerEvent) {
    event.stopPropagation();
    const startX = event.clientX;
    const startY = event.clientY;
    const origin = { ...place, ...box };
    const move = (next: PointerEvent) => {
      layout.patch(id, {
        w: Math.min(window.innerWidth - 24, Math.max(240, origin.w + next.clientX - startX)),
        h: Math.min(window.innerHeight - 24, Math.max(160, origin.h + next.clientY - startY)),
      });
    };
    track(event, move);
  }

  function track(event: PointerEvent, move: (next: PointerEvent) => void) {
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);
    const end = () => {
      target.removeEventListener('pointermove', move);
      target.removeEventListener('pointerup', end);
      target.removeEventListener('pointercancel', end);
    };
    target.addEventListener('pointermove', move);
    target.addEventListener('pointerup', end);
    target.addEventListener('pointercancel', end);
  }

  function moved(origin: PanelPlace, dx: number, dy: number): Partial<PanelPlace> {
    return panelBounds({ ...origin, x: origin.x + dx, y: origin.y + dy }, window.innerWidth, window.innerHeight);
  }
</script>

<svelte:window bind:innerWidth={viewportWidth} bind:innerHeight={viewportHeight} />

<section
  bind:this={panelElement}
  use:floatWindow={place.floating}
  class="panel"
  class:grow
  class:floating={place.floating}
  class:frontmost={place.floating && layout.frontmostWindow === id}
  class:collapsed={place.collapsed}
  data-panel={id}
  aria-labelledby={headingId}
  style={place.floating
    ? `--panel-stack:${layout.windowDepth(id)};left:${box.x}px;top:${box.y}px;width:${box.w}px;${place.collapsed ? '' : `height:${box.h}px;`}`
    : ''}
>
  {#if place.floating}<span id={instructionsId} class="sr-only">Drag to move, or use arrow keys. Hold Shift for larger steps.</span>{/if}
  <header bind:this={titleElement} class="panel-title" role="toolbar" aria-label={`${title} panel`} aria-describedby={place.floating ? instructionsId : undefined} tabindex="0" onpointerdown={drag} onkeydown={moveKeys}>
    <span class="panel-emblem" aria-hidden="true"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.35" stroke-linecap="round" stroke-linejoin="round"><path d={emblem} /></svg></span>
    <div class="panel-heading">
      {#if caption}<span class="panel-caption">{caption}</span>{/if}
      <span id={headingId} class="panel-title-text">{title}</span>
    </div>
    <div class="panel-controls" role="group" aria-label={`${title} window controls`}>
      <button class="panel-control" type="button" title={place.collapsed ? `Expand ${title}` : `Collapse ${title}`} aria-label={place.collapsed ? `Expand ${title}` : `Collapse ${title}`} aria-expanded={!place.collapsed} onclick={() => layout.patch(id, { collapsed: !place.collapsed })}>
        <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d={place.collapsed ? 'M4 10h12 M10 4v12' : 'M4 10h12'} /></svg>
      </button>
      {#if dockable}<button class="panel-control" type="button" title={place.floating ? 'Dock panel' : 'Pop out panel'} aria-label={place.floating ? `Dock ${title} panel` : `Pop out ${title}`} onclick={() => layout.patch(id, { floating: !place.floating })}>
        <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true"><path d={place.floating ? 'M4 4h12v12H4z M10 6v6 M6 12h6 M6 12l7-7' : 'M8 4H4v12h12v-4 M10 3h7v7 M17 3l-8 8'} /></svg>
      </button>{/if}
      {#if onclose}
        <button class="panel-control panel-close" type="button" title={`Close ${title} (Esc)`} aria-label={`Close ${title}`} onclick={close}><svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M5 5l10 10 M15 5L5 15" /></svg></button>
      {/if}
    </div>
  </header>
  {#if !place.collapsed}
    <div class="panel-body">
      {@render children()}
    </div>
  {/if}
  {#if place.floating && !place.collapsed}
    <button class="panel-resize" type="button" title="Drag to resize, or use arrow keys" aria-label={`Resize ${title} panel`} onpointerdown={resize} onkeydown={resizeKeys}><svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1" aria-hidden="true"><path d="M4 13l9-9 M8 13l5-5 M12 13l1-1" /></svg></button>
  {/if}
</section>
