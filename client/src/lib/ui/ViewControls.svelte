<script lang="ts">
  import { onMount } from 'svelte';

  let { onZoom, onFit, onMe, onCycle, onSetGrid }: {
    onZoom: (factor: number) => void;
    onFit: () => void;
    onMe: () => void;
    onCycle: () => 'off' | 'faint' | 'strong';
    onSetGrid: (style: 'off' | 'faint' | 'strong') => void;
  } = $props();

  const STORAGE_KEY = 'game-table.view';

  onMount(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'off' || saved === 'faint' || saved === 'strong') onSetGrid(saved);
  });

  function onKey(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    if (target?.closest('input, textarea, select')) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === '=' || event.key === '+') onZoom(0.85);
    else if (event.key === '-' || event.key === '_') onZoom(1.18);
    else if (event.key === '0') onFit();
    else if (event.key === 'c' || event.key === 'C') onMe();
    else if (event.key === 'g' || event.key === 'G') remember();
  }

  function remember() {
    const next = onCycle();
    localStorage.setItem(STORAGE_KEY, next);
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="pointer-events-auto flex items-center gap-1">
  <button class="btn quiet px-2 py-1" type="button" aria-label="Zoom out" onclick={() => onZoom(1.18)}>−</button>
  <button class="btn quiet px-2 py-1" type="button" aria-label="Zoom in" onclick={() => onZoom(0.85)}>+</button>
  <button class="btn quiet px-2 py-1" type="button" onclick={onFit}>Fit</button>
  <button class="btn quiet px-2 py-1" type="button" onclick={onMe}>Me</button>
  <button class="btn quiet px-2 py-1" type="button" onclick={remember}>Grid</button>
</div>
