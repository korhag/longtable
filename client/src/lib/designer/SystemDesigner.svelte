<script lang="ts">
  import type { Diagonal } from '../net/bindings/Diagonal.ts';
  import type { FieldKind } from '../net/bindings/FieldKind.ts';
  import { send } from '../net/socket';
  import { table } from '../table.svelte';
  import { designer, moveItem } from './designer.svelte';

  let freshId = $state('');
  let freshName = $state('');

  const kinds: FieldKind[] = ['number', 'text', 'track', 'pips'];
  const diagonals: Diagonal[] = ['square', 'alternate', 'euclidean'];

  function save() {
    if (!designer.draft) return;
    send({ type: 'saveSystem', system: designer.draft });
  }

</script>

<div class="flex flex-col gap-2 text-xs">
  <p class="text-xs tracking-[0.18em] text-[var(--color-gold)] uppercase">Edit</p>
  <select class="field" value={designer.draft?.id ?? ''} onchange={(event) => {
    const found = table.systems.find((system) => system.id === event.currentTarget.value);
    if (found) designer.open(found);
  }}>
    <option value="">Choose a system</option>
    {#each table.systems as system (system.id)}
      <option value={system.id}>{system.name}</option>
    {/each}
  </select>
  <div class="flex gap-1">
    <input class="field w-24" bind:value={freshId} placeholder="id" aria-label="New system id" />
    <input class="field min-w-0 flex-1" bind:value={freshName} placeholder="Name" aria-label="New system name" />
    <button class="btn quiet px-2" type="button" onclick={() => designer.blank(freshId.trim().toLowerCase(), freshName.trim() || 'New system')}>New</button>
  </div>
  {#if designer.draft}
    <button class="btn quiet" type="button" onclick={() => designer.blank(`${designer.draft?.id ?? 'copy'}2`, `${designer.draft?.name ?? ''} copy`)}>Copy</button>

    <label>Name
      <input class="field mt-1" value={designer.draft.name} oninput={(event) => { if (designer.draft) designer.draft.name = event.currentTarget.value; }} />
    </label>

    <details open>
      <summary>Fields</summary>
      {#each designer.draft.fields as field, index (field.id + index)}
        <div class="mt-2 rounded-lg border border-white/10 p-2">
          <div class="flex gap-1">
            <input class="field w-20" value={field.id} aria-label="Field id" onchange={(event) => { field.id = event.currentTarget.value.trim().toLowerCase(); }} />
            <input class="field min-w-0 flex-1" value={field.name} aria-label="Field name" oninput={(event) => { field.name = event.currentTarget.value; }} />
            <select class="field w-20" value={field.kind} onchange={(event) => { field.kind = event.currentTarget.value as FieldKind; }}>
              {#each kinds as kind (kind)}<option value={kind}>{kind}</option>{/each}
            </select>
          </div>
          <div class="mt-1 flex gap-1">
            <input class="field w-14" type="number" value={field.min} aria-label="Minimum" onchange={(event) => { field.min = Number(event.currentTarget.value); }} />
            <input class="field w-14" type="number" value={field.max ?? ''} placeholder="max" aria-label="Maximum" onchange={(event) => { field.max = event.currentTarget.value === '' ? null : Number(event.currentTarget.value); }} />
            <input class="field min-w-0 flex-1" value={field.maxFrom ?? ''} placeholder="max from" aria-label="Maximum from" onchange={(event) => { field.maxFrom = event.currentTarget.value || null; }} />
            <input class="field w-14" type="number" value={field.default} aria-label="Default" onchange={(event) => { field.default = Number(event.currentTarget.value); }} />
          </div>
          <div class="mt-1 flex gap-1">
            <button type="button" onclick={() => { if (designer.draft) designer.draft.fields = moveItem(designer.draft.fields, index, -1); }}>Up</button>
            <button type="button" onclick={() => { if (designer.draft) designer.draft.fields = moveItem(designer.draft.fields, index, 1); }}>Down</button>
            <button type="button" onclick={() => { if (designer.draft) designer.draft.fields = designer.draft.fields.filter((_, item) => item !== index); }}>Remove</button>
          </div>
        </div>
      {/each}
      <button class="mt-2" type="button" onclick={() => designer.draft?.fields.push({ id: `field${(designer.draft?.fields.length ?? 0) + 1}`, name: 'New', kind: 'number', min: 0, max: null, maxFrom: null, default: 0, textDefault: '' })}>Add field</button>
    </details>

    <details>
      <summary>Dice tags</summary>
      {#each designer.draft.dieTags as tag, index (index)}
        <div class="mt-2 flex gap-1">
          <input class="field w-10" maxlength="1" value={tag.letter} aria-label="Letter" oninput={(event) => { tag.letter = event.currentTarget.value.toLowerCase().slice(0, 1); }} />
          <input class="field min-w-0 flex-1" value={tag.name} aria-label="Tag name" oninput={(event) => { tag.name = event.currentTarget.value; }} />
          <input class="field w-20" value={tag.body} aria-label="Color" oninput={(event) => { tag.body = event.currentTarget.value; }} />
          <button type="button" onclick={() => { if (designer.draft) designer.draft.dieTags = designer.draft.dieTags.filter((_, item) => item !== index); }}>✕</button>
        </div>
      {/each}
      <button class="mt-2" type="button" onclick={() => designer.draft?.dieTags.push({ letter: 'a', id: `tag${(designer.draft?.dieTags.length ?? 0) + 1}`, name: 'Tag', body: '#d4a656', ink: '#1c140e' })}>Add tag</button>
    </details>

    <details>
      <summary>Actions</summary>
      {#each designer.draft.actions as action, index (index)}
        <div class="mt-2">
          <div class="flex gap-1">
            <input class="field w-20" value={action.id} aria-label="Action id" onchange={(event) => { action.id = event.currentTarget.value.trim().toLowerCase(); }} />
            <input class="field min-w-0 flex-1" value={action.name} aria-label="Action name" oninput={(event) => { action.name = event.currentTarget.value; }} />
          </div>
          <input class="field mt-1" value={action.formula} aria-label="Formula" oninput={(event) => { action.formula = event.currentTarget.value; }} />
          <button type="button" onclick={() => { if (designer.draft) designer.draft.actions = designer.draft.actions.filter((_, item) => item !== index); }}>Remove</button>
        </div>
      {/each}
      <button class="mt-2" type="button" onclick={() => designer.draft?.actions.push({ id: `action${(designer.draft?.actions.length ?? 0) + 1}`, name: 'Roll', formula: '1d20' })}>Add action</button>
    </details>

    <details>
      <summary>Sheet</summary>
      {#each designer.draft.sheet.sections as section, index (index)}
        <div class="mt-2">
          <input class="field" value={section.title} aria-label="Section" oninput={(event) => { section.title = event.currentTarget.value; }} />
          <input class="field mt-1" type="number" min="1" max="4" value={section.columns} aria-label="Columns" onchange={(event) => { section.columns = Number(event.currentTarget.value); }} />
          <input class="field mt-1" value={section.fields.join(', ')} aria-label="Fields" onchange={(event) => { section.fields = event.currentTarget.value.split(',').map((id) => id.trim()).filter(Boolean); }} />
          <button type="button" onclick={() => { if (designer.draft) designer.draft.sheet.sections = designer.draft.sheet.sections.filter((_, item) => item !== index); }}>Remove</button>
        </div>
      {/each}
      <button class="mt-2" type="button" onclick={() => designer.draft?.sheet.sections.push({ title: 'Section', columns: 2, fields: [] })}>Add section</button>
    </details>

    <details>
      <summary>Toolbar</summary>
      <input class="field mt-2" value={designer.draft.toolbar.resources.join(', ')} aria-label="Resources" onchange={(event) => { if (designer.draft) designer.draft.toolbar.resources = event.currentTarget.value.split(',').map((id) => id.trim()).filter(Boolean); }} />
      <input class="field mt-1" value={designer.draft.toolbar.actions.join(', ')} aria-label="Toolbar actions" onchange={(event) => { if (designer.draft) designer.draft.toolbar.actions = event.currentTarget.value.split(',').map((id) => id.trim()).filter(Boolean); }} />
    </details>

    <details>
      <summary>Grid</summary>
      {#if designer.draft.grid}
        <div class="mt-2 flex gap-1">
          <input class="field w-16" type="number" value={designer.draft.grid.perSquare} aria-label="Per square" onchange={(event) => { if (designer.draft?.grid) designer.draft.grid.perSquare = Number(event.currentTarget.value); }} />
          <input class="field w-16" value={designer.draft.grid.unit} aria-label="Unit" oninput={(event) => { if (designer.draft?.grid) designer.draft.grid.unit = event.currentTarget.value; }} />
          <select class="field" value={designer.draft.grid.diagonal} onchange={(event) => { if (designer.draft?.grid) designer.draft.grid.diagonal = event.currentTarget.value as Diagonal; }}>
            {#each diagonals as diagonal (diagonal)}<option value={diagonal}>{diagonal}</option>{/each}
          </select>
        </div>
        {#each designer.draft.grid.ranges as band, index (index)}
          <div class="mt-1 flex gap-1">
            <input class="field flex-1" value={band.name} aria-label="Range" oninput={(event) => { band.name = event.currentTarget.value; }} />
            <input class="field w-16" type="number" value={band.upTo} aria-label="Up to" onchange={(event) => { band.upTo = Number(event.currentTarget.value); }} />
          </div>
        {/each}
        <button class="mt-1" type="button" onclick={() => designer.draft?.grid?.ranges.push({ name: 'Close', upTo: 6 })}>Add range</button>
      {:else}
        <button class="mt-2" type="button" onclick={() => { if (designer.draft) designer.draft.grid = { perSquare: 5, unit: 'ft', diagonal: 'alternate', ranges: [] }; }}>Use a grid rule</button>
      {/if}
    </details>

    <div class="mt-2 flex gap-2">
      <button class="btn" type="button" onclick={save}>Save</button>
      <button class="btn quiet" type="button" onclick={() => designer.revert()}>Revert</button>
    </div>
  {/if}
</div>
