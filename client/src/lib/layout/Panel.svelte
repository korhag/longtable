<script lang="ts">
  import type { Snippet } from 'svelte';
  import { layout, type PanelPlace } from './layout.svelte';
  import { panelBounds } from './panelBounds';

  let { id, title, children, onclose, grow = false }: { id: string; title: string; children: Snippet; onclose?: () => void; grow?: boolean } = $props();

  let viewportWidth = $state(1400);
  let viewportHeight = $state(900);
  const place = $derived(layout.place(id));
  const box = $derived(panelBounds(place, viewportWidth, viewportHeight));

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
    if (!place.floating) return;
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
  class="panel"
  class:grow
  class:floating={place.floating}
  class:collapsed={place.collapsed}
  style={place.floating
    ? `left:${box.x}px;top:${box.y}px;width:${box.w}px;${place.collapsed ? '' : `height:${box.h}px;`}`
    : ''}
>
  <header class="panel-title" role="toolbar" aria-label={`${title} panel`} tabindex="0" onpointerdown={drag} onkeydown={moveKeys}>
    <span class="min-w-0 flex-1 truncate">{title}</span>
    <button type="button" aria-label={place.collapsed ? `Expand ${title}` : `Collapse ${title}`} aria-expanded={!place.collapsed} onclick={() => layout.patch(id, { collapsed: !place.collapsed })}>
      {place.collapsed ? '+' : '–'}
    </button>
    <button type="button" title={place.floating ? 'Dock panel' : 'Pop out'} aria-label={place.floating ? `Dock ${title} panel` : `Pop out ${title}`} onclick={() => layout.patch(id, { floating: !place.floating })}>
      {place.floating ? '↙' : '↗'}
    </button>
    {#if onclose}
      <button type="button" aria-label="Close" onclick={onclose}>✕</button>
    {/if}
  </header>
  {#if !place.collapsed}
    <div class="panel-body">
      {@render children()}
    </div>
  {/if}
  {#if place.floating && !place.collapsed}
    <button class="panel-resize" type="button" aria-label="Resize panel" onpointerdown={resize} onkeydown={resizeKeys}></button>
  {/if}
</section>
