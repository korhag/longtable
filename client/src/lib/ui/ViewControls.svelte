<script lang="ts">
  import { onMount } from 'svelte';

  import { send } from '../net/socket';
  import { table } from '../table.svelte';

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

  const presentation = $derived(table.scene?.presentation ?? null);
  const canTurn = $derived((presentation?.viewOrder.length ?? 0) > 1);
  const gm = $derived(table.role === 'gm' && !table.viewAs);
</script>

<svelte:window onkeydown={onKey} />

<div class="pointer-events-auto flex items-center gap-1">
  {#if canTurn}
    <button class="btn quiet px-2 py-1" type="button" aria-label="Turn left" onclick={() => table.turnView(-1)}>↺</button>
    <button class="btn quiet px-2 py-1" type="button" aria-label="Turn right" onclick={() => table.turnView(1)}>↻</button>
  {/if}
  {#if gm && presentation && presentation.lightings.length > 0}
    <select
      class="field w-28"
      aria-label="Lighting"
      value={presentation.activeLighting}
      onchange={(event) => send({ type: 'setLighting', lighting: event.currentTarget.value })}
    >
      {#each presentation.lightings as lighting (lighting.id)}
        <option value={lighting.id}>{lighting.label}</option>
      {/each}
    </select>
  {/if}
  <button class="btn quiet px-2 py-1" type="button" aria-label="Zoom out" onclick={() => onZoom(1.18)}>−</button>
  <button class="btn quiet px-2 py-1" type="button" aria-label="Zoom in" onclick={() => onZoom(0.85)}>+</button>
  <button class="btn quiet px-2 py-1" type="button" onclick={onFit}>Fit</button>
  <button class="btn quiet px-2 py-1" type="button" onclick={onMe}>Me</button>
  <button class="btn quiet px-2 py-1" type="button" onclick={remember}>Grid</button>
</div>
