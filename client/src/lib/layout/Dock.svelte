<script lang="ts">
  import type { Snippet } from 'svelte';
  import { layout } from './layout.svelte';
  import { modes } from './modes.svelte';

  let { side, children }: { side: 'left' | 'right'; children: Snippet } = $props();

  const open = $derived(side === 'left' ? layout.leftOpen : layout.rightOpen);
  const label = $derived(side === 'right' ? 'Dice & rolls' : modes.current === 'play' ? 'Game tools' : modes.current === 'map' ? 'Map studio' : modes.current === 'campaign' ? 'Campaign' : 'Rules');
  const width = $derived(side === 'left' ? layout.leftWidth : layout.rightWidth);

  function drag(event: PointerEvent) {
    const start = event.clientX;
    const origin = width;
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);
    const move = (next: PointerEvent) => {
      const delta = next.clientX - start;
      layout.setDock(side, side === 'left' ? origin + delta : origin - delta);
    };
    const end = () => {
      target.removeEventListener('pointermove', move);
      target.removeEventListener('pointerup', end);
      target.removeEventListener('pointercancel', end);
    };
    target.addEventListener('pointermove', move);
    target.addEventListener('pointerup', end);
    target.addEventListener('pointercancel', end);
  }
</script>

<aside class="dock" class:right={side === 'right'} class:open aria-label={label} style={`--dock-width:${width}px`}>
  <div class="dock-rail">
    <button class="rail-toggle" type="button" aria-label={`${open ? 'Close' : 'Open'} ${label.toLowerCase()}`} aria-expanded={open} title={label} onclick={() => layout.toggleDock(side)}>
      <span aria-hidden="true">{side === 'left' ? '☷' : '⚄'}</span>
      <span class="rail-label">{label}</span>
    </button>
  </div>
  {#if open}
    <div class="dock-stack">{@render children()}</div>
    <button class="dock-grip" type="button" aria-label={`Resize ${label.toLowerCase()}`} onpointerdown={drag} onkeydown={(event) => {
      if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
        event.preventDefault();
        layout.setDock(side, width + (event.key === 'ArrowRight' ? 20 : -20) * (side === 'left' ? 1 : -1));
      }
    }}></button>
  {/if}
</aside>
