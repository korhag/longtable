<script lang="ts">
  import type { Snippet } from 'svelte';
  import { layout } from './layout.svelte';

  let { side, children }: { side: 'left' | 'right'; children: Snippet } = $props();

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
    };
    target.addEventListener('pointermove', move);
    target.addEventListener('pointerup', end);
  }
</script>

<aside class="dock" class:right={side === 'right'} style={`width:${width}px`}>
  {#if side === 'right'}
    <button class="dock-grip" type="button" aria-label="Resize dock" onpointerdown={drag}></button>
  {/if}
  <div class="dock-stack">
    {@render children()}
  </div>
  {#if side === 'left'}
    <button class="dock-grip" type="button" aria-label="Resize dock" onpointerdown={drag}></button>
  {/if}
</aside>
