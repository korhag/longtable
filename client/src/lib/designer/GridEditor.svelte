<script lang="ts">
  import type { Diagonal } from '../net/bindings/Diagonal.ts';
  import { designer } from './designer.svelte';

  const diagonals: { id: Diagonal; label: string }[] = [
    { id: 'square', label: 'Diagonal costs 1' },
    { id: 'alternate', label: 'Every other diagonal costs 2' },
    { id: 'euclidean', label: 'Straight-line distance' },
  ];
</script>

<div class="flex flex-col gap-2">
  <p class="text-[var(--color-parchment)]/70">How a dragged measurement turns into squares, and the names of those distances.</p>
  {#if designer.draft}
    {#if designer.draft.grid}
      <label>Distance per square
        <input class="field mt-1" type="number" bind:value={designer.draft.grid.perSquare} />
      </label>
      <label>Unit, such as ft
        <input class="field mt-1" bind:value={designer.draft.grid.unit} />
      </label>
      <label>Diagonals
        <select class="field mt-1" bind:value={designer.draft.grid.diagonal}>
          {#each diagonals as diagonal (diagonal.id)}
            <option value={diagonal.id}>{diagonal.label}</option>
          {/each}
        </select>
      </label>
      <p class="mt-2 text-[var(--color-gold)] uppercase tracking-[0.14em]">Ranges</p>
      <p class="text-[var(--color-parchment)]/60">The first range that reaches the square count is the name shown while dragging.</p>
      {#each designer.draft.grid.ranges as band, index (index)}
        <div class="flex gap-1">
          <input class="field" bind:value={band.name} aria-label="Range name" />
          <input class="field w-20" type="number" bind:value={band.upTo} aria-label="Up to squares" />
          <button class="btn quiet px-2" type="button" onclick={() => {
            if (designer.draft?.grid) designer.draft.grid.ranges = designer.draft.grid.ranges.filter((_, item) => item !== index);
          }}>Remove</button>
        </div>
      {/each}
      <button class="btn quiet" type="button" onclick={() => designer.draft?.grid?.ranges.push({ name: 'Close', upTo: 6 })}>Add a range</button>
      <button class="btn quiet" type="button" onclick={() => { if (designer.draft) designer.draft.grid = null; }}>Remove the grid rule</button>
    {:else}
      <p class="text-[var(--color-parchment)]/50">This system does not name distances. Measurements stay in squares.</p>
      <button class="btn" type="button" onclick={() => {
        if (designer.draft) designer.draft.grid = { perSquare: 5, unit: 'ft', diagonal: 'alternate', ranges: [] };
      }}>Use a grid rule</button>
    {/if}
  {/if}
</div>
