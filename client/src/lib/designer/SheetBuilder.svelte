<script lang="ts">
  import type { Character } from '../net/bindings/Character.ts';
  import CharacterSheet from '../ui/CharacterSheet.svelte';
  import { designer, moveItem } from './designer.svelte';

  let numbers = $state<Record<string, number>>({});
  let texts = $state<Record<string, string>>({});
  let sampleFor = $state('');
  let dragging = $state<string | null>(null);

  const kind = $derived(designer.view === 'overview' ? 'pc' : designer.view);
  const sheet = $derived(designer.draft ? designer.draft.sheets[kind] : null);

  $effect(() => {
    const id = designer.draft?.id ?? '';
    if (sampleFor !== id) {
      sampleFor = id;
      numbers = {};
      texts = {};
    }
  });

  const sample = $derived.by((): Character | null => {
    const system = designer.draft;
    if (!system) return null;
    const nextNumbers: Record<string, number> = {};
    const nextTexts: Record<string, string> = {};
    for (const field of system.fields) {
      if (field.kind === 'text' || field.kind === 'choice') nextTexts[field.id] = texts[field.id] ?? field.textDefault;
      else if (field.kind === 'dice') nextTexts[field.id] = texts[field.id] ?? field.formula;
      else if (field.kind === 'thresholds') {
        for (const step of field.steps) nextNumbers[step.id] = numbers[step.id] ?? step.default;
      } else nextNumbers[field.id] = numbers[field.id] ?? field.default;
    }
    return {
      id: 'sample',
      name: 'Sample',
      kind,
      ownerId: null,
      systemId: system.id,
      numbers: { ...nextNumbers, ...numbers },
      texts: { ...nextTexts, ...texts },
    };
  });

  function preview(next: Character) {
    numbers = next.numbers;
    texts = next.texts;
  }

  function addSection() {
    if (!designer.draft) return;
    const current = designer.draft.sheets[kind];
    current.sections = [...current.sections, { title: 'Section', columns: 2, fields: [] }];
  }

  function removeSection(index: number) {
    designer.removeSection(kind, index);
  }

  function shift(sectionIndex: number, fieldIndex: number, delta: number) {
    if (!designer.draft) return;
    const section = designer.draft.sheets[kind].sections[sectionIndex];
    if (!section) return;
    section.fields = moveItem(section.fields, fieldIndex, delta);
  }

  function dropOn(event: DragEvent, sectionIndex: number, beforeId: string | null) {
    event.preventDefault();
    const id = event.dataTransfer?.getData('text/plain') || dragging;
    if (!id) return;
    designer.move(kind, id, sectionIndex, beforeId);
    dragging = null;
  }

  function toggleBar(list: 'resources' | 'actions', id: string) {
    if (!designer.draft) return;
    const toolbar = designer.draft.sheets[kind].toolbar;
    const current = toolbar[list];
    toolbar[list] = current.includes(id) ? current.filter((item) => item !== id) : [...current, id];
  }
</script>

{#if designer.draft && sheet && sample}
  <div class="grid min-h-0 grid-cols-1 gap-3 xl:grid-cols-2">
    <div class="flex min-h-0 flex-col gap-2">
      <p class="text-[12px] text-[var(--color-parchment)]/60">
        {kind === 'adversary' ? 'Arrange the adversary card.' : `Arrange the ${kind === 'pc' ? 'PC' : 'NPC'} sheet.`}
        Drag a thing, or add one to a section.
      </p>
      {#each sheet.sections as section, sectionIndex (sectionIndex)}
        <section
          class="rounded-lg border border-white/10 p-2"
          role="group"
          aria-label={section.title || 'Section'}
          ondragover={(event) => event.preventDefault()}
          ondrop={(event) => dropOn(event, sectionIndex, null)}
        >
          <label>Section
            <input class="field mt-1" bind:value={section.title} />
          </label>
          <label class="mt-1 block">Columns
            <input class="field mt-1 w-20" type="number" min="1" max="4" bind:value={section.columns} />
          </label>
          <div class="mt-2 flex flex-col gap-1">
            {#each section.fields as id, fieldIndex (id)}
              {@const field = designer.draft.fields.find((item) => item.id === id)}
              {#if field}
                <div
                  class="flex flex-wrap items-center gap-1 rounded-lg border border-white/10 p-1"
                  role="group"
                  aria-label={field.name}
                  ondragover={(event) => event.preventDefault()}
                  ondrop={(event) => { event.stopPropagation(); dropOn(event, sectionIndex, id); }}
                >
                  <button
                    class="btn quiet px-2"
                    type="button"
                    draggable="true"
                    aria-label={`Drag ${field.name}`}
                    ondragstart={(event) => { dragging = id; event.dataTransfer?.setData('text/plain', id); }}
                  >Drag</button>
                  <span class="min-w-0 flex-1 truncate text-sm">{field.name}</span>
                  <button class="btn quiet px-2" type="button" onclick={() => shift(sectionIndex, fieldIndex, -1)}>Up</button>
                  <button class="btn quiet px-2" type="button" onclick={() => shift(sectionIndex, fieldIndex, 1)}>Down</button>
                  <button class="btn quiet px-2" type="button" onclick={() => designer.unplace(kind, id)}>Remove</button>
                </div>
              {/if}
            {/each}
          </div>
          <label class="mt-2 block">Add a thing
            <select class="field mt-1" onchange={(event) => {
              const id = event.currentTarget.value;
              if (id) designer.move(kind, id, sectionIndex, null);
              event.currentTarget.value = '';
            }}>
              <option value="">Choose</option>
              {#each designer.draft.fields.filter((field) => !section.fields.includes(field.id)) as field (field.id)}
                <option value={field.id}>{field.name}</option>
              {/each}
            </select>
          </label>
          <button class="btn quiet mt-2" type="button" onclick={() => removeSection(sectionIndex)}>Remove section</button>
        </section>
      {/each}
      <button class="btn quiet" type="button" onclick={addSection}>Add a section</button>

      <div class="rounded-lg border border-[rgba(212,166,86,0.3)] p-2">
        <p class="text-[12px] tracking-[0.14em] text-[var(--color-gold)] uppercase">Character bar</p>
        <p class="mt-1 text-[var(--color-parchment)]/60">These show on the bar while this kind of character is selected.</p>
        <p class="mt-2">Resources</p>
        {#each designer.draft.fields.filter((field) => (field.kind === 'pool' || field.kind === 'number') && sheet.sections.some((section) => section.fields.includes(field.id))) as field (field.id)}
          <label class="mt-1 flex min-h-8 items-center gap-2">
            <input class="h-8 w-8" type="checkbox" checked={sheet.toolbar.resources.includes(field.id)} onchange={() => toggleBar('resources', field.id)} />
            {field.name}
          </label>
        {/each}
        <p class="mt-2">Rolls</p>
        {#each designer.draft.actions as action (action.id)}
          <label class="mt-1 flex min-h-8 items-center gap-2">
            <input class="h-8 w-8" type="checkbox" checked={sheet.toolbar.actions.includes(action.id)} onchange={() => toggleBar('actions', action.id)} />
            {action.name}
          </label>
        {/each}
        {#each designer.draft.fields.filter((field) => field.kind === 'dice' && sheet.sections.some((section) => section.fields.includes(field.id))) as field (field.id)}
          <label class="mt-1 flex min-h-8 items-center gap-2">
            <input class="h-8 w-8" type="checkbox" checked={sheet.toolbar.actions.includes(field.id)} onchange={() => toggleBar('actions', field.id)} />
            {field.name}
          </label>
        {/each}
      </div>
    </div>

    <div class="min-h-0 overflow-auto rounded-xl border border-[rgba(212,166,86,0.3)] p-3">
      <p class="mb-2 text-[12px] tracking-[0.14em] text-[var(--color-gold)] uppercase">How it looks</p>
      <CharacterSheet character={sample} system={designer.draft} preview onpreview={preview} />
    </div>
  </div>
{/if}
