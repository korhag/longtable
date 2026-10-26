<script lang="ts">
  import type { ActionDef } from '../net/bindings/ActionDef.ts';
  import { designer } from './designer.svelte';
  import { takenIds, thingId } from './sheets';

  let focused: HTMLInputElement | null = null;

  const refs = $derived.by(() => {
    const system = designer.draft;
    if (!system) return [];
    const found: { id: string; name: string }[] = [];
    for (const field of system.fields) {
      if (field.kind === 'number' || field.kind === 'pool') found.push({ id: field.id, name: field.name });
      if (field.kind === 'thresholds') {
        for (const step of field.steps) found.push({ id: step.id, name: `${field.name}: ${step.name}` });
      }
    }
    return found;
  });

  function insert(action: ActionDef, id: string) {
    if (!id) return;
    const token = `@${id}`;
    const start = focused?.selectionStart ?? action.formula.length;
    const end = focused?.selectionEnd ?? start;
    action.formula = action.formula.slice(0, start) + token + action.formula.slice(end);
  }

  function addAction() {
    if (!designer.draft) return;
    const id = thingId('Roll', takenIds(designer.draft));
    designer.draft.actions = [...designer.draft.actions, { id, name: 'Roll', formula: '1d20' }];
  }

  function addTag() {
    if (!designer.draft) return;
    const id = thingId('Tag', takenIds(designer.draft));
    designer.draft.dieTags = [...designer.draft.dieTags, { letter: 'a', id, name: 'Tag', body: '#d4a656', ink: '#1c140e' }];
  }
</script>

<div class="flex flex-col gap-3">
  <section>
    <p class="text-[var(--color-gold)] uppercase tracking-[0.14em]">Rolls</p>
    <p class="mt-1 text-[var(--color-parchment)]/70">A named roll. @strength reads that thing from the character.</p>
    {#if designer.draft}
      {#each designer.draft.actions as action (action.id)}
        <div class="mt-2 rounded-lg border border-white/10 p-2">
          <label>Name
            <input class="field mt-1" bind:value={action.name} />
          </label>
          <label class="mt-1 block">Formula
            <input class="field mt-1" bind:value={action.formula} onfocus={(event) => { focused = event.currentTarget; }} />
          </label>
          <label class="mt-1 block">Insert a thing
            <select class="field mt-1" onchange={(event) => { insert(action, event.currentTarget.value); event.currentTarget.value = ''; }}>
              <option value="">Choose</option>
              {#each refs as ref (ref.id)}
                <option value={ref.id}>{ref.name}</option>
              {/each}
            </select>
          </label>
          <button class="btn quiet mt-2" type="button" onclick={() => {
            if (designer.draft) designer.draft.actions = designer.draft.actions.filter((item) => item.id !== action.id);
          }}>Remove roll</button>
        </div>
      {/each}
      <button class="btn mt-2" type="button" onclick={addAction}>Add a roll</button>
    {/if}
  </section>

  <section>
    <p class="text-[var(--color-gold)] uppercase tracking-[0.14em]">Special dice</p>
    <p class="mt-1 text-[var(--color-parchment)]/70">A letter painted on a die. 1d12h uses the letter h.</p>
    {#if designer.draft}
      {#each designer.draft.dieTags as tag (tag.id)}
        <div class="mt-2 rounded-lg border border-white/10 p-2">
          <div class="flex gap-1">
            <label class="w-14">Letter
              <input class="field mt-1" maxlength="1" value={tag.letter} oninput={(event) => { tag.letter = event.currentTarget.value.toLowerCase().slice(0, 1); }} />
            </label>
            <label class="min-w-0 flex-1">Name
              <input class="field mt-1" bind:value={tag.name} />
            </label>
          </div>
          <div class="mt-1 flex gap-1">
            <label class="flex-1">Die color
              <input class="field mt-1 h-8" type="color" bind:value={tag.body} />
            </label>
            <label class="flex-1">Number color
              <input class="field mt-1 h-8" type="color" bind:value={tag.ink} />
            </label>
          </div>
          <button class="btn quiet mt-2" type="button" onclick={() => {
            if (designer.draft) designer.draft.dieTags = designer.draft.dieTags.filter((item) => item.id !== tag.id);
          }}>Remove die</button>
        </div>
      {/each}
      <button class="btn quiet mt-2" type="button" onclick={addTag}>Add a special die</button>
    {/if}
  </section>
</div>
