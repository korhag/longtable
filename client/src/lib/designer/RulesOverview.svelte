<script lang="ts">
  import { designer } from './designer.svelte';
  import { describeFormula, describeThing } from './describe';
  import { kindTitle, onSheet, SHEET_KINDS } from './sheets';

  const system = $derived(designer.draft);
  const loose = $derived(
    system?.fields.filter((field) => !SHEET_KINDS.some((sheet) => onSheet(system.sheets, sheet.kind, field.id))) ?? [],
  );

  function carried(id: string): string {
    if (!system) return '';
    const names = SHEET_KINDS.filter((sheet) => onSheet(system.sheets, sheet.kind, id)).map((sheet) =>
      sheet.kind === 'adversary' ? 'Adversary' : sheet.title,
    );
    return names.length === 0 ? 'On no sheet.' : `On ${names.join(', ')}.`;
  }
</script>

{#if system}
  <div class="flex flex-col gap-4 text-[12px]">
    {#if loose.length > 0}
      <p class="text-[#e7a090]">{loose.map((field) => field.name).join(', ')} {loose.length === 1 ? 'is' : 'are'} not on a sheet yet.</p>
    {/if}

    <section>
      <p class="tracking-[0.14em] text-[var(--color-gold)] uppercase">Things</p>
      <div class="mt-2 grid gap-2 md:grid-cols-2">
        {#each system.fields as field (field.id)}
          <button class="btn quiet text-left" type="button" onclick={() => designer.focusThing(field.id)}>
            <span class="block text-sm">{field.name}</span>
            <span class="block font-normal text-[var(--color-parchment)]/50">{kindTitle(field.kind)}</span>
            <span class="mt-1 block font-normal text-[var(--color-parchment)]/80">{describeThing(field, system)}</span>
            <span class="mt-1 block font-normal text-[var(--color-gold)]/80">{carried(field.id)}</span>
          </button>
        {:else}
          <p class="text-[var(--color-parchment)]/50">No things yet. Add one from the left.</p>
        {/each}
      </div>
    </section>

    <section>
      <p class="tracking-[0.14em] text-[var(--color-gold)] uppercase">Rolls</p>
      <div class="mt-2 flex flex-col gap-2">
        {#each system.actions as action (action.id)}
          <div class="rounded-lg border border-white/10 p-2">
            <p class="text-sm">{action.name}</p>
            <p class="text-[var(--color-parchment)]/75">{describeFormula(action.formula, system)}</p>
          </div>
        {:else}
          <p class="text-[var(--color-parchment)]/50">No named rolls yet.</p>
        {/each}
      </div>
    </section>

    <section>
      <p class="tracking-[0.14em] text-[var(--color-gold)] uppercase">Special dice</p>
      <div class="mt-2 flex flex-wrap gap-2">
        {#each system.dieTags as tag (tag.id)}
          <span class="inline-flex items-center gap-2">
            <span class="inline-flex h-8 min-w-8 items-center justify-center rounded-md px-2 font-bold" style={`background:${tag.body};color:${tag.ink}`}>{tag.letter}</span>
            <span>{tag.name}</span>
          </span>
        {:else}
          <p class="text-[var(--color-parchment)]/50">No special dice. Ordinary dice still roll.</p>
        {/each}
      </div>
    </section>

    <section>
      <p class="tracking-[0.14em] text-[var(--color-gold)] uppercase">Distances</p>
      {#if system.grid}
        <p class="mt-2 text-[var(--color-parchment)]/80">
          {system.grid.perSquare}{system.grid.unit ? ` ${system.grid.unit}` : ''} per square.
          {system.grid.diagonal === 'square' ? 'A diagonal costs 1.' : system.grid.diagonal === 'alternate' ? 'Every other diagonal costs 2.' : 'Distance is a straight line.'}
        </p>
        {#if system.grid.ranges.length > 0}
          <p class="text-[var(--color-parchment)]/70">
            {system.grid.ranges.map((band) => `${band.name} up to ${band.upTo}`).join(' · ')}
          </p>
        {/if}
      {:else}
        <p class="mt-2 text-[var(--color-parchment)]/60">Measurements stay in squares.</p>
      {/if}
    </section>
  </div>
{/if}
