<script lang="ts">
  import type { Character } from '../net/bindings/Character.ts';
  import type { FieldDef } from '../net/bindings/FieldDef.ts';
  import type { SystemDef } from '../net/bindings/SystemDef.ts';
  import { send } from '../net/socket';
  import FieldWidget from './FieldWidget.svelte';

  let { character, system, preview = false, onpreview }: {
    character: Character;
    system: SystemDef;
    preview?: boolean;
    onpreview?: (next: Character) => void;
  } = $props();

  const sheet = $derived(system.sheets[character.kind]);
  const card = $derived(character.kind === 'adversary');
  const typeLine = $derived.by(() => {
    if (!card) return null;
    for (const section of sheet.sections) {
      for (const id of section.fields) {
        const field = definition(id);
        if (field?.kind === 'choice') return character.texts[id] || field.options[0] || '';
      }
    }
    return null;
  });

  function definition(id: string): FieldDef | undefined {
    return system.fields.find((field) => field.id === id);
  }

  function setNumber(id: string, value: number) {
    if (preview) onpreview?.({ ...character, numbers: { ...character.numbers, [id]: value } });
    else send({ type: 'updateCharacter', characterId: character.id, name: null, kind: null, ownerId: null, clearOwner: false, numbers: { [id]: value }, texts: {} });
  }

  function setText(id: string, value: string) {
    if (preview) onpreview?.({ ...character, texts: { ...character.texts, [id]: value } });
    else send({ type: 'updateCharacter', characterId: character.id, name: null, kind: null, ownerId: null, clearOwner: false, numbers: {}, texts: { [id]: value } });
  }
</script>

<div class="flex flex-col gap-3 {card ? 'rounded-xl border border-[rgba(212,166,86,0.45)] bg-[#1a120c] p-3' : ''}">
  {#if card}
    <header>
      <p class="display text-lg">{character.name}</p>
      {#if typeLine}<p class="text-[12px] tracking-[0.14em] text-[var(--color-gold)] uppercase">{typeLine}</p>{/if}
    </header>
  {/if}
  {#if sheet.sections.length === 0}
    <p class="text-[12px] text-[var(--color-parchment)]/50">Nothing on this sheet yet.</p>
  {/if}
  {#each sheet.sections as section, index (index)}
    <section>
      <p class="text-[12px] tracking-[0.16em] text-[var(--color-gold)] uppercase">{section.title}</p>
      <div class="mt-1 grid gap-2" style={`grid-template-columns:repeat(${section.columns},minmax(0,1fr))`}>
        {#each section.fields as id (id)}
          {@const field = definition(id)}
          {#if field}
            <FieldWidget {field} {character} {system} {preview} onnumber={setNumber} ontext={setText} />
          {/if}
        {/each}
      </div>
    </section>
  {/each}
</div>
