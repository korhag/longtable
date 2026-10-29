<script lang="ts">
  import { send } from '../net/socket';
  import { table } from '../table.svelte';
  import { designer, type DesignerStep } from './designer.svelte';
  import GridEditor from './GridEditor.svelte';
  import RollsEditor from './RollsEditor.svelte';
  import ThingsEditor from './ThingsEditor.svelte';

  let freshName = $state('');
  let pending = $state<(() => void) | null>(null);

  const steps: { id: DesignerStep; title: string; line: string }[] = [
    { id: 'things', title: 'Things', line: 'The values this game tracks, such as HP, Stress, and Damage.' },
    { id: 'rolls', title: 'Rolls', line: 'Named rolls built from those things, and dice with their own colors.' },
    { id: 'sheets', title: 'Sheets', line: 'Which things appear for a PC, an NPC, and an adversary.' },
    { id: 'table', title: 'Table', line: 'How the grid measures distance while you drag.' },
  ];

  function guard(action: () => void) {
    if (designer.dirty) pending = action;
    else action();
  }

  function save() {
    if (!designer.draft) return;
    send({ type: 'saveSystem', system: $state.snapshot(designer.draft) });
  }
</script>

<div class="flex min-h-full flex-col gap-3 text-sm">
  <div class="flex flex-col gap-2">
    {#if designer.draft}
      <p class="tracking-[0.14em] text-[var(--color-gold)] uppercase">{designer.creating ? 'Creating' : 'Editing'}</p>
      <p class="display text-base">{designer.draft.name}</p>
      {#if designer.dirty}<p class="text-[var(--color-gold-bright)]">Unsaved changes</p>{/if}
    {:else}
      <p class="tracking-[0.14em] text-[var(--color-gold)] uppercase">Rules</p>
      <p class="text-[var(--color-parchment)]/70">Load a rule set to work on it, or start a new one.</p>
    {/if}

    <div class="flex flex-col gap-2">
      {#each table.systems as system (system.id)}
        <div class="rounded-lg border p-2 {designer.draft?.id === system.id ? 'border-[rgba(212,166,86,0.8)]' : 'border-white/10'}">
          <p class="text-sm">{system.name}</p>
          <p class="text-[var(--color-parchment)]/60">{system.fields.length} things · {system.actions.length} rolls</p>
          <div class="mt-1 flex flex-wrap items-center gap-1">
            {#if table.campaign.systemId === system.id}
              <span class="text-[var(--color-gold)]">In this campaign</span>
            {/if}
            {#if designer.draft?.id === system.id}
              <span class="text-[var(--color-gold-bright)]">Loaded</span>
            {:else}
              <button class="btn quiet px-2" type="button" onclick={() => guard(() => designer.load(system))}>Load</button>
            {/if}
          </div>
        </div>
      {:else}
        <p class="text-[var(--color-parchment)]/50">No rule sets on this computer yet.</p>
      {/each}
    </div>

    {#if designer.notice}
      <p class={designer.notice.tone === 'ok' ? 'text-[var(--color-gold)]' : 'text-[#e7a090]'}>{designer.notice.text}</p>
    {/if}

    <label>New system name
      <input class="field mt-1" bind:value={freshName} placeholder="Such as Daggerheart" />
    </label>
    <button class="btn quiet" type="button" onclick={() => guard(() => designer.blank(freshName))}>New system</button>
    {#if designer.draft}
      <button class="btn quiet" type="button" onclick={() => designer.duplicate()}>Duplicate</button>
    {/if}

    {#if pending}
      <div class="rounded-lg border border-[rgba(212,166,86,0.35)] p-2">
        <p>Discard unsaved changes and continue?</p>
        <div class="mt-2 flex gap-1">
          <button class="btn quiet flex-1" type="button" onclick={() => { pending = null; }}>Stay</button>
          <button class="btn flex-1" type="button" onclick={() => {
            const next = pending;
            pending = null;
            designer.revert();
            next?.();
          }}>Discard</button>
        </div>
      </div>
    {/if}
  </div>

  {#if designer.draft}
    <div class="flex flex-col gap-1">
      {#each steps as step (step.id)}
        <button class="btn quiet text-left" class:active={designer.step === step.id} type="button" onclick={() => { designer.step = step.id; }}>
          <span class="block">{step.title}</span>
          <span class="block font-normal text-[var(--color-parchment)]/60">{step.line}</span>
        </button>
      {/each}
    </div>

    {#if designer.step === 'things'}
      <ThingsEditor />
    {:else if designer.step === 'rolls'}
      <RollsEditor />
    {:else if designer.step === 'sheets'}
      <p class="text-[var(--color-parchment)]/70">Use the sheet tabs in the middle. The chips on each thing do the same job.</p>
    {:else}
      <GridEditor />
    {/if}

    <div class="sticky bottom-0 mt-auto flex gap-2 bg-[#140e0a] py-2">
      <button class="btn flex-1" type="button" onclick={save}>Save</button>
      <button class="btn quiet" type="button" onclick={() => designer.revert()}>Discard</button>
    </div>
  {/if}
</div>
