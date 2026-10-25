<script lang="ts">
  import type { Character } from '../net/bindings/Character.ts';
  import type { FieldDef } from '../net/bindings/FieldDef.ts';
  import type { SystemDef } from '../net/bindings/SystemDef.ts';
  import { send } from '../net/socket';

  let { character, system, preview = false, onpreview }: {
    character: Character;
    system: SystemDef;
    preview?: boolean;
    onpreview?: (next: Character) => void;
  } = $props();

  function definition(id: string) {
    return system.fields.find((field) => field.id === id);
  }

  function ceiling(field: FieldDef) {
    if (field.maxFrom) return Math.max(0, character.numbers[field.maxFrom] ?? 0);
    return field.max ?? 1000;
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

<div class="flex flex-col gap-3">
  {#each system.sheet.sections as section (section.title)}
    <section>
      <p class="text-[10px] tracking-[0.16em] text-[var(--color-gold)] uppercase">{section.title}</p>
      <div class="mt-1 grid gap-2" style={`grid-template-columns:repeat(${section.columns},minmax(0,1fr))`}>
        {#each section.fields as id (id)}
          {@const field = definition(id)}
          {#if field}
            <label class="text-[11px]">
              <span class="text-[var(--color-parchment)]/60">{field.name}</span>
              {#if field.kind === 'text'}
                <textarea class="field mt-1 min-h-16" value={character.texts[id] ?? ''} onchange={(event) => setText(id, event.currentTarget.value)}></textarea>
              {:else}
                <input
                  class="field mt-1"
                  type="number"
                  min={field.kind === 'number' ? field.min : 0}
                  max={ceiling(field)}
                  value={character.numbers[id] ?? 0}
                  onchange={(event) => setNumber(id, Math.trunc(Number(event.currentTarget.value) || 0))}
                />
                {#if field.kind !== 'number'}
                  <span class="text-[10px] text-[var(--color-parchment)]/45">of {ceiling(field)}</span>
                {/if}
              {/if}
            </label>
          {/if}
        {/each}
      </div>
    </section>
  {/each}
</div>
