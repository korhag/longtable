<script lang="ts">
  import type { Character } from '../net/bindings/Character.ts';
  import CharacterBar from '../ui/CharacterBar.svelte';
  import CharacterSheet from '../ui/CharacterSheet.svelte';
  import { designer } from './designer.svelte';

  const sample = $derived.by((): Character | null => {
    const system = designer.draft;
    if (!system) return null;
    const numbers: Record<string, number> = {};
    const texts: Record<string, string> = {};
    for (const field of system.fields) {
      if (field.kind === 'text') texts[field.id] = field.textDefault;
      else numbers[field.id] = field.default;
    }
    return { id: 'sample', name: system.name || 'Sample', kind: 'pc', ownerId: null, systemId: system.id, numbers, texts };
  });
</script>

{#if designer.draft && sample}
  <div class="pointer-events-auto max-h-full max-w-xl overflow-auto rounded-2xl border border-[rgba(212,166,86,0.3)] bg-[#1c140e]/90 p-4">
    <p class="mb-2 text-xs tracking-[0.16em] text-[var(--color-gold)] uppercase">Preview</p>
    <CharacterSheet character={sample} system={designer.draft} preview />
    <div class="mt-3">
      <CharacterBar preview={{ character: sample, system: designer.draft }} />
    </div>
  </div>
{/if}
