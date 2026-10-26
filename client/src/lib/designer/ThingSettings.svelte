<script lang="ts">
  import type { FieldDef } from '../net/bindings/FieldDef.ts';
  import type { PoolLook } from '../net/bindings/PoolLook.ts';
  import { designer } from './designer.svelte';
  import { takenIds, thingId } from './sheets';

  let { field }: { field: FieldDef } = $props();

  const numbers = $derived(
    (designer.draft?.fields ?? []).filter((item) => item.id !== field.id && (item.kind === 'number' || item.kind === 'pool')),
  );
  const pools = $derived((designer.draft?.fields ?? []).filter((item) => item.kind === 'pool'));
  const looks: { id: PoolLook; label: string }[] = [
    { id: 'boxes', label: 'Boxes' },
    { id: 'pips', label: 'Pips' },
    { id: 'count', label: 'Count' },
  ];

  function addStep() {
    if (!designer.draft) return;
    const step = { id: thingId('Step', takenIds(designer.draft)), name: 'Step', default: 1, marks: 1 };
    field.steps = [...field.steps, step];
  }

  function addOption() {
    field.options = [...field.options, `Option ${field.options.length + 1}`];
  }
</script>

<div class="mt-2 flex flex-col gap-2 border-t border-white/10 pt-2">
  <label>Name
    <input class="field mt-1" bind:value={field.name} />
  </label>
  <p class="text-[var(--color-parchment)]/50">Use in formulas as @{field.id}</p>
  <label>Hint on the sheet
    <input class="field mt-1" bind:value={field.hint} placeholder="Optional" />
  </label>

  {#if field.kind === 'number' || field.kind === 'pool'}
    <div class="flex gap-1">
      <label class="w-20">Min
        <input class="field mt-1" type="number" bind:value={field.min} />
      </label>
      <label class="w-20">Start
        <input class="field mt-1" type="number" bind:value={field.default} />
      </label>
      <label class="min-w-0 flex-1">Maximum
        <input
          class="field mt-1"
          type="number"
          value={field.max ?? ''}
          onchange={(event) => { field.max = event.currentTarget.value === '' ? null : Number(event.currentTarget.value); }}
        />
      </label>
    </div>
  {/if}

  {#if field.kind === 'pool'}
    <label>Draw it as
      <select class="field mt-1" bind:value={field.look}>
        {#each looks as look (look.id)}
          <option value={look.id}>{look.label}</option>
        {/each}
      </select>
    </label>
    <p class="text-[var(--color-parchment)]/50">Boxes and pips are marks you click. Count is a plain number.</p>
    <label>Take the maximum from another thing
      <select class="field mt-1" value={field.maxFrom ?? ''} onchange={(event) => { field.maxFrom = event.currentTarget.value || null; }}>
        <option value="">Use the maximum above</option>
        {#each numbers as source (source.id)}
          <option value={source.id}>{source.name}</option>
        {/each}
      </select>
    </label>
    <p class="text-[var(--color-parchment)]/50">When the character has that thing, it wins. Otherwise the maximum above is used.</p>
  {/if}

  {#if field.kind === 'text'}
    <label>Starting text
      <textarea class="field mt-1 min-h-16" bind:value={field.textDefault}></textarea>
    </label>
  {/if}

  {#if field.kind === 'choice'}
    <p>Options</p>
    {#each field.options as option, index (index)}
      <div class="flex gap-1">
        <input class="field" value={option} aria-label="Option" oninput={(event) => { field.options[index] = event.currentTarget.value; }} />
        <button class="btn quiet px-2" type="button" onclick={() => { field.options = field.options.filter((_, item) => item !== index); }}>Remove</button>
      </div>
    {/each}
    <button class="btn quiet" type="button" onclick={addOption}>Add option</button>
    <label>Starts as
      <select class="field mt-1" bind:value={field.textDefault}>
        <option value="">First option</option>
        {#each field.options as option (option)}
          <option value={option}>{option}</option>
        {/each}
      </select>
    </label>
  {/if}

  {#if field.kind === 'thresholds'}
    <label>Marks this pool
      <select class="field mt-1" value={field.pool ?? ''} onchange={(event) => { field.pool = event.currentTarget.value || null; }}>
        <option value="">Choose a pool</option>
        {#each pools as pool (pool.id)}
          <option value={pool.id}>{pool.name}</option>
        {/each}
      </select>
    </label>
    {#if pools.length === 0}
      <p class="text-[var(--color-parchment)]/50">Add a pool first, such as HP. Apply damage marks that pool.</p>
    {/if}
    {#each field.steps as step (step.id)}
      <div class="rounded-lg border border-white/10 p-2">
        <label>Step name
          <input class="field mt-1" bind:value={step.name} />
        </label>
        <p class="mt-1 text-[var(--color-parchment)]/50">Use in formulas as @{step.id}</p>
        <div class="mt-1 flex gap-1">
          <label class="w-20">At
            <input class="field mt-1" type="number" bind:value={step.default} />
          </label>
          <label class="w-20">Marks
            <input class="field mt-1" type="number" min="0" bind:value={step.marks} />
          </label>
        </div>
        <button class="btn quiet mt-1" type="button" onclick={() => { field.steps = field.steps.filter((item) => item.id !== step.id); }}>Remove step</button>
      </div>
    {/each}
    <button class="btn quiet" type="button" onclick={addStep}>Add step</button>
    <p class="text-[var(--color-parchment)]/50">Apply uses the highest step whose value is at most the damage. Below the first step, nothing is marked.</p>
  {/if}

  {#if field.kind === 'dice'}
    <label>Starting formula
      <input class="field mt-1" bind:value={field.formula} />
    </label>
    <p class="text-[var(--color-parchment)]/50">Each character can change this on their sheet. @strength reads that thing.</p>
  {/if}
</div>
