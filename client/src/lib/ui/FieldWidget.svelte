<script lang="ts">
  import { rollSetup } from '../dice/rollSetup.svelte';
  import type { Character } from '../net/bindings/Character.ts';
  import type { FieldDef } from '../net/bindings/FieldDef.ts';
  import type { SystemDef } from '../net/bindings/SystemDef.ts';
  import { send } from '../net/socket';
  import { ceiling, fillFormula, thresholdMarks } from '../systems/fields';

  let { field, character, system, preview = false, onnumber, ontext }: {
    field: FieldDef;
    character: Character;
    system: SystemDef;
    preview?: boolean;
    onnumber: (id: string, value: number) => void;
    ontext: (id: string, value: string) => void;
  } = $props();

  let amount = $state(0);
  let note = $state('');

  const cap = $derived(ceiling(field, character.numbers));
  const value = $derived(character.numbers[field.id] ?? field.default);
  const formula = $derived(character.texts[field.id] ?? field.formula);
  const marks = $derived(Math.min(cap, 12));

  function setAmount(next: number) {
    note = '';
    const whole = Math.trunc(Number.isFinite(next) ? next : 0);
    if (field.kind === 'pool') onnumber(field.id, Math.min(cap, Math.max(0, whole)));
    else onnumber(field.id, whole);
  }

  function apply() {
    const dealt = Math.max(0, Math.trunc(amount));
    const stepMarks = thresholdMarks(field, dealt, character.numbers);
    if (stepMarks === null) {
      note = 'Below the lowest step, nothing is marked.';
      return;
    }
    note = '';
    if (preview) {
      const pool = system.fields.find((item) => item.id === field.pool);
      if (!pool || !field.pool) return;
      const current = character.numbers[field.pool] ?? 0;
      const next = Math.min(ceiling(pool, character.numbers), Math.max(0, current + stepMarks));
      onnumber(field.pool, next);
      return;
    }
    send({ type: 'applyDamage', characterId: character.id, fieldId: field.id, amount: dealt });
  }

  function roll() {
    if (preview) return;
    send({
      type: 'rollAction',
      characterId: character.id,
      actionId: field.id,
      visibility: rollSetup.visibility,
      plus: rollSetup.plus,
      minus: rollSetup.minus,
    });
  }
</script>

<div class="flex flex-col gap-1 text-[12px]">
  <span class="text-[var(--color-parchment)]/70">{field.name}</span>
  {#if field.kind === 'text'}
    <textarea class="field min-h-16" value={character.texts[field.id] ?? ''} onchange={(event) => ontext(field.id, event.currentTarget.value)}></textarea>
  {:else if field.kind === 'choice'}
    <select class="field" value={character.texts[field.id] ?? field.options[0] ?? ''} onchange={(event) => ontext(field.id, event.currentTarget.value)}>
      {#each field.options as option, index (index)}
        <option value={option}>{option}</option>
      {/each}
    </select>
  {:else if field.kind === 'number'}
    <input class="field" type="number" min={field.min} max={field.max ?? undefined} {value} onchange={(event) => setAmount(Number(event.currentTarget.value) || 0)} />
  {:else if field.kind === 'pool'}
    {#if field.look === 'count' || cap > 12}
      <div class="flex items-center gap-1">
        <button class="btn quiet px-2" type="button" aria-label={`Decrease ${field.name}`} onclick={() => setAmount(value - 1)}>−</button>
        <span class="display text-sm">{value} / {cap}</span>
        <button class="btn quiet px-2" type="button" aria-label={`Increase ${field.name}`} onclick={() => setAmount(value + 1)}>+</button>
      </div>
    {:else if cap === 0}
      <span class="text-[var(--color-parchment)]/45">No slots yet</span>
    {:else}
      <div class="flex flex-wrap gap-1" role="group" aria-label={field.name}>
        {#each Array.from({ length: marks }, (_, index) => index) as index (index)}
          <button
            class="h-8 w-8 border border-[rgba(212,166,86,0.45)] {field.look === 'pips' ? 'rounded-full' : 'rounded-md'} {index < value ? 'bg-[var(--color-gold)]' : 'bg-transparent'}"
            type="button"
            aria-label={`${field.name} ${index + 1}`}
            onclick={() => setAmount(index < value && index === value - 1 ? index : index + 1)}
          ></button>
        {/each}
      </div>
      <span class="text-[var(--color-parchment)]/45">{value} / {cap}</span>
    {/if}
  {:else if field.kind === 'thresholds'}
    {#each field.steps as step (step.id)}
      <label class="flex items-center gap-2">
        <span class="w-16 shrink-0">{step.name}</span>
        <input
          class="field w-16"
          type="number"
          value={character.numbers[step.id] ?? step.default}
          aria-label={step.name}
          onchange={(event) => onnumber(step.id, Math.trunc(Number(event.currentTarget.value) || 0))}
        />
        <span class="text-[var(--color-parchment)]/45">marks {step.marks}</span>
      </label>
    {/each}
    <div class="mt-1 flex items-center gap-1">
      <input class="field w-16" type="number" min="0" bind:value={amount} aria-label="Damage" />
      <button class="btn" type="button" onclick={apply}>Apply</button>
    </div>
    {#if note}<p class="text-[var(--color-parchment)]/60">{note}</p>{/if}
  {:else if field.kind === 'dice'}
    <input class="field" value={formula} aria-label={`${field.name} formula`} onchange={(event) => ontext(field.id, event.currentTarget.value)} />
    {#if preview}
      <p class="text-[var(--color-parchment)]/55">{fillFormula(formula, character.numbers)}</p>
    {:else}
      <button class="btn quiet" type="button" onclick={roll}>Roll</button>
    {/if}
  {/if}
  {#if field.hint}<p class="text-[var(--color-parchment)]/45">{field.hint}</p>{/if}
</div>
