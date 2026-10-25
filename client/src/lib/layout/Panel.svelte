<script lang="ts">
  import type { Snippet } from 'svelte';
  import { layout, type PanelPlace } from './layout.svelte';

  let { id, title, children, onclose, grow = false }: { id: string; title: string; children: Snippet; onclose?: () => void; grow?: boolean } = $props();

  const place = $derived(layout.place(id));

  function drag(event: PointerEvent) {
    if (!place.floating) return;
    if ((event.target as HTMLElement).closest('button')) return;
    const startX = event.clientX;
    const startY = event.clientY;
    const origin = { ...place };
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
    const origin = { ...place };
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
    };
    target.addEventListener('pointermove', move);
    target.addEventListener('pointerup', end);
  }

  function moved(origin: PanelPlace, dx: number, dy: number): Partial<PanelPlace> {
    const w = origin.w;
    const h = origin.collapsed ? 44 : origin.h;
    return {
      x: Math.min(window.innerWidth - 80, Math.max(8, origin.x + dx)),
      y: Math.min(window.innerHeight - 40, Math.max(8, origin.y + dy)),
      w: Math.min(w, window.innerWidth - 16),
      h: Math.min(h, window.innerHeight - 16),
    };
  }
</script>

<section
  class="panel"
  class:grow
  class:floating={place.floating}
  class:collapsed={place.collapsed}
  style={place.floating
    ? `left:${place.x}px;top:${place.y}px;width:${place.w}px;${place.collapsed ? '' : `height:${place.h}px;`}`
    : ''}
>
  <header class="panel-title" role="toolbar" tabindex="0" onpointerdown={drag}>
    <span class="min-w-0 flex-1 truncate">{title}</span>
    <button type="button" aria-label={place.collapsed ? 'Expand' : 'Collapse'} onclick={() => layout.patch(id, { collapsed: !place.collapsed })}>
      {place.collapsed ? '+' : '–'}
    </button>
    <button type="button" aria-label={place.floating ? 'Dock panel' : 'Pop out'} onclick={() => layout.patch(id, { floating: !place.floating })}>
      {place.floating ? 'Dock' : 'Pop out'}
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
    <button class="panel-resize" type="button" aria-label="Resize panel" onpointerdown={resize}></button>
  {/if}
</section>
