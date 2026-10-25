<script lang="ts">
  import type { RollLogEntry } from '../net/bindings/RollLogEntry.ts';
  import { systems } from '../systems/store.svelte';
  import { table } from '../table.svelte';

  const entries = $derived([...table.rolls].reverse());

  function open(entry: RollLogEntry) {
    if (entry.concealed) return false;
    if (table.activeRoll?.id === entry.id && table.landedRollId !== entry.id) return false;
    return true;
  }

  function dieText(entry: RollLogEntry) {
    return entry.dice
      .map((die) => {
        const name = die.tag ? systems.active?.dieTags.find((tag) => tag.letter === die.tag)?.name : undefined;
        return `${name ? `${name} ` : ''}${die.sign < 0 ? '−' : ''}${die.value}`;
      })
      .join(' + ');
  }

  function title(entry: RollLogEntry) {
    const text = entry.label || entry.formula || '';
    return text.length > 28 ? `${text.slice(0, 27)}…` : text;
  }
</script>

<aside class="pointer-events-none flex h-full flex-col gap-1 overflow-y-auto pr-1">
  {#each entries.slice(0, 20) as entry (entry.id)}
    <article class="glass pointer-events-auto rounded-xl px-2.5 py-1.5">
      <div class="flex items-baseline gap-2">
        <p class="shrink-0 text-xs">{entry.rollerName}</p>
        <p class="min-w-0 flex-1 truncate text-[11px] text-[var(--color-parchment)]/65">
          {open(entry) ? title(entry) : '···'}
        </p>
        {#if open(entry) && entry.total !== null}
          <p class="display text-base leading-none text-[var(--color-gold-bright)]">{entry.total}</p>
        {:else if entry.concealed}
          <p class="text-[10px] tracking-wide text-[var(--color-gold)] uppercase">Hidden</p>
        {:else}
          <p class="text-[10px] tracking-wide text-[var(--color-gold)] uppercase">Rolling</p>
        {/if}
      </div>
      {#if entry.concealed}
        <p class="text-[11px] text-[var(--color-parchment)]/60">{entry.notice}</p>
      {:else if open(entry)}
        <p class="truncate text-sm text-[var(--color-parchment)]/70">
          {dieText(entry)}
          {#if entry.outcome}
            <span class="text-[var(--color-gold-bright)]"> · {entry.outcome.label}</span>
          {/if}
        </p>
      {/if}
    </article>
  {/each}
</aside>
