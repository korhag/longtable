<script lang="ts">
  import { modes } from '../../lib/layout/modes.svelte';
  import { systems } from '../../lib/systems/store.svelte';
  import { table } from '../../lib/table.svelte';
  import { ADVERSARIES, ADVERSARY_TYPES, JAGGED_KNIFE_BANDIT } from './catalog';
  import { creation } from './creation.svelte';
  import { adversaryPayload, previewAdversary, type AdversaryDraft } from './draft';
  import AdversaryCard from './AdversaryCard.svelte';
  import { createFrom, placeCharacter } from './save';
  import './daggerheart.css';

  let draft = $state<AdversaryDraft>({
    name: '',
    type: 'Standard',
    tier: 1,
    description: '',
    motives: '',
    difficulty: 12,
    major: 8,
    severe: 14,
    hp: 5,
    stress: 3,
    attack: 1,
    attackName: 'Daggers',
    attackRange: 'Melee',
    damage: '1d8+1',
    experience: '',
    features: '',
    place: true,
  });
  let busy = $state(false);
  let error = $state('');
  let query = $state('');

  const matches = $derived(
    ADVERSARIES.filter((item) => item.name.toLowerCase().includes(query.trim().toLowerCase())).slice(0, 40),
  );

  function useAdversary(name: string) {
    const found = ADVERSARIES.find((item) => item.name === name) ?? JAGGED_KNIFE_BANDIT;
    draft = { ...draft, ...found, place: draft.place };
  }

  async function finish() {
    if (busy || draft.name.trim() === '') return;
    busy = true;
    error = '';
    try {
      const body = adversaryPayload(draft);
      const id = await createFrom('adversary', body);
      if (draft.place) placeCharacter(id, body.name, 'adversary');
      systems.sheetOpen = true;
      creation.close();
      if (draft.place) modes.set('play');
    } catch (caught) {
      error = caught instanceof Error ? caught.message : 'Could not create that adversary.';
    } finally {
      busy = false;
    }
  }
</script>

<div class="dh-overlay dh-adversary-maker">
  <div class="dh-rail">
    <div class="flex items-center justify-between px-1">
      <p class="dh-rail-title display text-sm text-[var(--color-gold-bright)]">Adversary folio<span>New adversary</span></p>
      <button class="btn quiet px-2" type="button" onclick={() => creation.close()}>Close</button>
    </div>
    <label class="mt-3 block">Find in the book
      <input class="field mt-1" bind:value={query} placeholder="Acid Burrower, Jagged Knife…" />
    </label>
    <div class="mt-2 max-h-48 overflow-auto">
      {#each matches as item (item.name)}
        <button class="flex w-full items-baseline justify-between gap-2 rounded px-1 py-1 text-left text-sm hover:bg-white/10" type="button" onclick={() => useAdversary(item.name)}>
          <span>{item.name}</span>
          <span class="text-[var(--color-gold)]">T{item.tier} {item.type}</span>
        </button>
      {/each}
    </div>
    <div class="mt-3 grid gap-2 overflow-auto pr-1">
      <label>Name
        <input class="field mt-1" bind:value={draft.name} maxlength="48" />
      </label>
      <label>Type
        <select class="field mt-1" bind:value={draft.type}>
          {#each ADVERSARY_TYPES as type (type)}
            <option value={type}>{type}</option>
          {/each}
        </select>
      </label>
      <div class="grid grid-cols-2 gap-2">
        <label>Tier <input class="field mt-1" type="number" min="1" max="4" bind:value={draft.tier} /></label>
        <label>Difficulty <input class="field mt-1" type="number" min="1" max="30" bind:value={draft.difficulty} /></label>
        <label>Major <input class="field mt-1" type="number" min="1" max="40" bind:value={draft.major} /></label>
        <label>Severe <input class="field mt-1" type="number" min="1" max="60" bind:value={draft.severe} /></label>
        <label>HP <input class="field mt-1" type="number" min="1" max="20" bind:value={draft.hp} /></label>
        <label>Stress <input class="field mt-1" type="number" min="0" max="20" bind:value={draft.stress} /></label>
        <label>Attack <input class="field mt-1" type="number" min="-5" max="12" bind:value={draft.attack} /></label>
        <label>Damage <input class="field mt-1" bind:value={draft.damage} maxlength="16" /></label>
      </div>
      <label>Description
        <textarea class="field mt-1 min-h-14" bind:value={draft.description} maxlength="240"></textarea>
      </label>
      <label>Motives & tactics
        <input class="field mt-1" bind:value={draft.motives} maxlength="160" />
      </label>
      <div class="grid grid-cols-2 gap-2">
        <label>Attack name <input class="field mt-1" bind:value={draft.attackName} maxlength="24" /></label>
        <label>Range <input class="field mt-1" bind:value={draft.attackRange} maxlength="16" /></label>
      </div>
      <label>Experience
        <input class="field mt-1" bind:value={draft.experience} maxlength="48" placeholder="Thief +2" />
      </label>
      <label>Features
        <textarea class="field mt-1 min-h-28" bind:value={draft.features} maxlength="4000" placeholder="Name — Passive: what it does."></textarea>
      </label>
      <label class="flex items-center gap-2">
        <input type="checkbox" bind:checked={draft.place} disabled={!table.scene} />
        Place on {table.scene?.name ?? 'the scene'}
      </label>
      <p class="text-sm text-[#e7a090]">{error}</p>
      <button class="btn" type="button" disabled={busy || draft.name.trim() === ''} onclick={() => void finish()}>{busy ? 'Writing…' : 'Create adversary'}</button>
    </div>
  </div>
  <div class="dh-body">
    <p class="mb-3 text-sm text-[var(--color-parchment)]/65">The card updates as you type. Hidden from players until you show the token.</p>
    <div class="max-w-xl">
      <AdversaryCard character={previewAdversary(draft)} preview />
    </div>
  </div>
</div>
