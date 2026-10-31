<script lang="ts">
  import { modes } from '../../lib/layout/modes.svelte';
  import { layout } from '../../lib/layout/layout.svelte';
  import { systems } from '../../lib/systems/store.svelte';
  import { table } from '../../lib/table.svelte';
  import {
    ANCESTRIES,
    ARMORS,
    CLASSES,
    COMMUNITIES,
    EXPERIENCE_EXAMPLES,
    OFFHAND,
    TRAITS,
    WEAPONS,
    ancestryByName,
    armorById,
    classById,
    communityByName,
    weaponById,
    type TraitId,
  } from './catalog';
  import { creation } from './creation.svelte';
  import {
    applyClass,
    emptyDraft,
    evasionOf,
    payload,
    previewCharacter,
    remainingModifiers,
    takenCards,
    traitsReady,
    useSuggestion,
    type CreationDraft,
  } from './draft';
  import PcSheet from './PcSheet.svelte';
  import { createFrom, placeCharacter } from './save';
  import './daggerheart.css';

  const genders = ['Male', 'Female'] as const;

  const steps = [
    { kicker: 'Step 1', title: 'Class' },
    { kicker: 'Step 1', title: 'Subclass' },
    { kicker: 'Step 2', title: 'Heritage' },
    { kicker: 'Step 2', title: 'Name' },
    { kicker: 'Step 3', title: 'Traits' },
    { kicker: 'Step 4', title: 'Record' },
    { kicker: 'Step 5', title: 'Weapons' },
    { kicker: 'Step 5', title: 'Armor' },
    { kicker: 'Step 6', title: 'Background' },
    { kicker: 'Step 7', title: 'Experiences' },
    { kicker: 'Step 8', title: 'Domains' },
    { kicker: 'Step 9', title: 'Connections' },
    { kicker: 'Ready', title: 'Review' },
  ];

  let step = $state(0);
  let draft = $state<CreationDraft>(emptyDraft());
  let held = $state<number | null>(null);
  let busy = $state(false);
  let error = $state('');

  const chosen = $derived(classById(draft.classId));
  const weapon = $derived(weaponById(draft.weaponId));
  const armor = $derived(armorById(draft.armorId));
  const bag = $derived(remainingModifiers(draft.traits, held));
  const ready = $derived(canLeave(step));
  const complete = $derived([0, 1, 2, 3, 4, 6, 7, 9, 10].every((index) => canLeave(index)));

  function canLeave(index: number): boolean {
    if (index === 0) return draft.classId !== '';
    if (index === 1) return draft.subclassId !== '';
    if (index === 2) return draft.ancestry !== '' && draft.community !== '';
    if (index === 3) return draft.name.trim().length > 0;
    if (index === 4) return traitsReady(draft.traits);
    if (index === 6) return draft.weaponId !== '' && (weapon?.hands === 2 || draft.secondaryId !== '');
    if (index === 7) return draft.armorId !== '' && draft.keepsake.trim() !== '';
    if (index === 9) return draft.experiences.every((item) => item.trim().length > 0);
    if (index === 10) return takenCards(draft).length === 2;
    return true;
  }

  function chooseClass(id: string) {
    draft = applyClass(draft, id);
    held = null;
  }

  function suggest() {
    draft = useSuggestion(draft);
    held = null;
  }

  function take(value: number) {
    held = held === value ? null : value;
  }

  function assign(id: TraitId) {
    if (held === null) {
      draft.traits[id] = null;
      return;
    }
    const previous = draft.traits[id];
    draft.traits[id] = held;
    held = previous;
  }

  function fillExperience(example: string) {
    const index = draft.experiences.findIndex((item) => item.trim() === '');
    if (index < 0) draft.experiences[1] = example;
    else draft.experiences[index] = example;
  }

  function toggleCard(id: string) {
    const card = draft.cards.find((item) => item.id === id);
    if (!card) return;
    if (!card.taken && takenCards(draft).length >= 2) return;
    card.taken = !card.taken;
  }

  async function finish() {
    if (busy) return;
    busy = true;
    error = '';
    try {
      const body = payload(draft);
      const id = await createFrom('pc', body);
      if (draft.place) placeCharacter(id, body.name, 'pc');
      systems.sheetOpen = true;
      const place = layout.place('sheet');
      layout.patch('sheet', { floating: true, collapsed: false, w: Math.max(place.w, 760), h: Math.max(place.h, 680) });
      creation.close();
      if (draft.place) modes.set('play');
    } catch (caught) {
      error = caught instanceof Error ? caught.message : 'Could not create that character.';
    } finally {
      busy = false;
    }
  }

  function signed(value: number) {
    return value > 0 ? `+${value}` : `${value}`;
  }
</script>

<div class="dh-overlay dh-character-creator">
  <aside class="dh-rail">
    <p class="dh-rail-title display text-sm text-[var(--color-gold-bright)]">Character folio<span>New adventurer</span></p>
    {#each steps as item, index (item.title)}
      <button class:on={step === index} class:done={index < step} type="button" aria-current={step === index ? 'step' : undefined} onclick={() => { if (index <= step) step = index; }}>
        <span class="mark">{index < step ? '✓' : index + 1}</span>
        <span>{item.title}</span>
        <span class="kicker">{item.kicker}</span>
      </button>
    {/each}
  </aside>

  <div class="dh-main">
    <header>
      <div class="min-w-0 flex-1">
        <p class="text-sm tracking-[0.18em] text-[var(--color-gold)] uppercase">{steps[step]?.kicker}</p>
        <h2 class="display text-2xl">{steps[step]?.title}</h2>
      </div>
      <button class="btn quiet" type="button" onclick={() => creation.close()}>Close</button>
    </header>

    <div class="dh-body">
      {#if step === 0}
        <p class="dh-muted mb-3 max-w-2xl text-sm">A class is the role you play. The subclass, and its foundation feature, come next.</p>
        <div class="dh-grid">
          {#each CLASSES as item (item.id)}
            <button class="dh-card" class:on={draft.classId === item.id} type="button" onclick={() => chooseClass(item.id)}>
              <p class="display text-lg">{item.name}</p>
              <p class="mt-1 text-sm text-[var(--color-gold)]">{item.domains[0]} & {item.domains[1]}</p>
              <p class="mt-2 text-sm">{item.pitch}</p>
              <p class="mt-2 text-sm">Evasion {item.evasion} · HP {item.hp} · Spellcast {item.spellcastLabel}</p>
            </button>
          {/each}
        </div>
        {#if chosen}
          <div class="mt-4 max-w-3xl whitespace-pre-wrap text-sm leading-relaxed">{chosen.features.join('\n\n')}</div>
          <p class="mt-3 max-w-3xl whitespace-pre-wrap text-sm leading-relaxed">{chosen.hope}</p>
        {/if}
      {:else if step === 1 && chosen}
        <p class="dh-muted mb-3 text-sm">A subclass sharpens the class. Pick one foundation.</p>
        <div class="dh-grid">
          {#each chosen.subclasses as item (item.id)}
            <button class="dh-card text-left" class:on={draft.subclassId === item.id} type="button" onclick={() => (draft.subclassId = item.id)}>
              <p class="display text-lg">{item.name}</p>
              <p class="mt-2 text-sm">{item.pitch}</p>
              {#if item.foundation}
                <p class="mt-2 max-h-36 overflow-auto whitespace-pre-wrap text-sm text-[var(--color-parchment)]/80">{item.foundation}</p>
              {/if}
            </button>
          {/each}
        </div>
      {:else if step === 2}
        <p class="dh-muted mb-3 text-sm">Heritage is an ancestry and a community. Each one grants the feature under the list.</p>
        <p class="text-sm tracking-[0.16em] text-[var(--color-gold)] uppercase">Ancestry</p>
        <div class="mt-2 flex flex-wrap gap-1.5">
          {#each ANCESTRIES as item (item.name)}
            <button class="dh-chip" class:on={draft.ancestry === item.name} type="button" onclick={() => (draft.ancestry = item.name)}>{item.name}</button>
          {/each}
        </div>
        {#if ancestryByName(draft.ancestry)}
          <p class="mt-2 max-w-2xl whitespace-pre-wrap text-sm">{ancestryByName(draft.ancestry)?.feature}</p>
        {/if}
        <p class="mt-4 text-sm tracking-[0.16em] text-[var(--color-gold)] uppercase">Community</p>
        <div class="mt-2 flex flex-wrap gap-1.5">
          {#each COMMUNITIES as item (item.name)}
            <button class="dh-chip" class:on={draft.community === item.name} type="button" onclick={() => (draft.community = item.name)}>{item.name}</button>
          {/each}
        </div>
        {#if communityByName(draft.community)}
          <p class="mt-2 max-w-2xl whitespace-pre-wrap text-sm">{communityByName(draft.community)?.feature}</p>
        {/if}
      {:else if step === 3}
        <div class="grid max-w-xl gap-3">
          <label>Name
            <input class="field mt-1" bind:value={draft.name} maxlength="24" placeholder="A name the table will use" />
          </label>
          <div>
            <p class="text-sm tracking-[0.16em] text-[var(--color-gold)] uppercase">Gender</p>
            <div class="mt-1 flex gap-1.5">
              {#each genders as gender (gender)}
                <button class="dh-chip" class:on={draft.gender === gender} type="button" onclick={() => (draft.gender = gender)}>{gender}</button>
              {/each}
            </div>
          </div>
          <label>Description
            <textarea class="field mt-1 min-h-28" bind:value={draft.description} maxlength="600" placeholder="Clothes, eyes, body, skin, and the way they carry themselves."></textarea>
          </label>
        </div>
      {:else if step === 4}
        <p class="dh-muted max-w-2xl text-sm">Place +2, +1, +1, 0, 0, and −1. A high trait is what this character reaches for. You can use the class suggestion, then move a chip.</p>
        <button class="btn quiet mt-3" type="button" onclick={suggest}>Use the {chosen?.name ?? 'class'} suggestion</button>
        <div class="mt-4 flex flex-wrap gap-2">
          {#each TRAITS as trait (trait.id)}
            <button class="dh-shield" type="button" onclick={() => assign(trait.id)}>
              <b>{draft.traits[trait.id] === null ? '·' : signed(draft.traits[trait.id] ?? 0)}</b>
              <span>{trait.name}</span>
            </button>
          {/each}
        </div>
        <p class="mt-4 text-sm tracking-[0.14em] text-[var(--color-gold)] uppercase">{held === null ? 'Chips left' : `Holding ${signed(held)} — click a trait`}</p>
        <div class="mt-2 flex flex-wrap gap-1.5">
          {#each bag as value, index (`${value}-${index}`)}
            <button class="dh-chip" class:on={held === value} type="button" onclick={() => take(value)}>{signed(value)}</button>
          {/each}
        </div>
      {:else if step === 5 && chosen}
        <p class="dh-muted max-w-2xl text-sm">These are written on the sheet for you. Armor, chosen next, can still move Evasion.</p>
        <div class="mt-4 flex flex-wrap gap-2">
          <div class="dh-shield"><b>{chosen.evasion}</b><span>Evasion</span></div>
          <div class="dh-shield"><b>{chosen.hp}</b><span>HP</span></div>
          <div class="dh-shield"><b>6</b><span>Stress</span></div>
          <div class="dh-shield"><b>2</b><span>Hope</span></div>
          <div class="dh-shield"><b>1</b><span>Level</span></div>
          <div class="dh-shield"><b>1</b><span>Proficiency</span></div>
        </div>
        <p class="mt-4 max-w-xl text-sm">Hope starts at 2 and can rise to 6. Stress starts empty, with 6 boxes. Hit points start unmarked. Proficiency 1 means one weapon die.</p>
      {:else if step === 6}
        <p class="dh-muted mb-3 text-sm">One two-handed weapon, or a one-handed weapon and something in the other hand. Proficiency is already in the damage die.</p>
        <div class="dh-grid">
          {#each WEAPONS as item (item.id)}
            <button class="dh-card" class:on={draft.weaponId === item.id} type="button" onclick={() => (draft.weaponId = item.id)}>
              <p class="display">{item.name}</p>
              <p class="mt-1 text-sm text-[var(--color-gold)]">{item.trait} · {item.range} · {item.hands === 2 ? 'Two-handed' : 'One-handed'}</p>
              <p class="mt-1 text-sm">{item.damage}</p>
            </button>
          {/each}
        </div>
        {#if weapon && weapon.hands === 1}
          <p class="mt-4 text-sm tracking-[0.16em] text-[var(--color-gold)] uppercase">Other hand</p>
          <div class="mt-2 flex flex-wrap gap-1.5">
            <button class="dh-chip" class:on={draft.secondaryId === 'open'} type="button" onclick={() => (draft.secondaryId = 'open')}>Open hand</button>
            {#each OFFHAND as item (item.id)}
              <button class="dh-chip" class:on={draft.secondaryId === item.id} type="button" onclick={() => (draft.secondaryId = item.id)}>{item.name} · {item.damage}</button>
            {/each}
          </div>
        {:else if weapon}
          <p class="mt-3 text-sm">Both hands are on the {weapon.name}.</p>
        {/if}
      {:else if step === 7 && chosen}
        <div class="dh-grid">
          {#each ARMORS as item (item.id)}
            <button class="dh-card" class:on={draft.armorId === item.id} type="button" onclick={() => (draft.armorId = item.id)}>
              <p class="display">{item.name}</p>
              <p class="mt-1 text-sm">Thresholds {item.major + 1}/{item.severe + 1} at level 1 · Score {item.score}</p>
              <p class="mt-1 text-sm">{item.note}</p>
            </button>
          {/each}
        </div>
        {#if armor}
          <p class="mt-3 text-sm">Evasion is {chosen.evasion} from the class{armor.evasion === 0 ? '' : `, ${armor.evasion > 0 ? '+' : '−'}${Math.abs(armor.evasion)} from ${armor.name}`}, so {evasionOf(draft)}.</p>
        {/if}
        <p class="mt-4 text-sm tracking-[0.16em] text-[var(--color-gold)] uppercase">Potion</p>
        <div class="mt-2 flex flex-wrap gap-1.5">
          <button class="dh-chip" class:on={draft.potion === 'health'} type="button" onclick={() => (draft.potion = 'health')}>Minor Health Potion</button>
          <button class="dh-chip" class:on={draft.potion === 'stamina'} type="button" onclick={() => (draft.potion = 'stamina')}>Minor Stamina Potion</button>
        </div>
        <p class="mt-4 text-sm tracking-[0.16em] text-[var(--color-gold)] uppercase">And either</p>
        <div class="mt-2 flex flex-wrap gap-1.5">
          {#each chosen.keepsakes as item (item)}
            <button class="dh-chip" class:on={draft.keepsake === item} type="button" onclick={() => (draft.keepsake = item)}>{item}</button>
          {/each}
        </div>
        <p class="mt-3 text-sm text-[var(--color-parchment)]/60">Everyone also starts with a torch, 50 feet of rope, basic supplies, and a handful of gold.</p>
      {:else if step === 8 && chosen}
        <p class="dh-muted mb-3 max-w-2xl text-sm">Answer what you want. A blank answer can wait until the table is talking.</p>
        <div class="grid max-w-2xl gap-3">
          {#each chosen.questions as question, index (question)}
            <label>{question}
              <textarea class="field mt-1 min-h-16" bind:value={draft.background[index]} maxlength="500"></textarea>
            </label>
          {/each}
        </div>
      {:else if step === 9}
        <p class="dh-muted mb-3 max-w-2xl text-sm">Two experiences, each +2. Specific is better than broad. They are not spells.</p>
        <div class="grid max-w-xl gap-2">
          <input class="field" bind:value={draft.experiences[0]} maxlength="48" placeholder="First experience" aria-label="First experience" />
          <input class="field" bind:value={draft.experiences[1]} maxlength="48" placeholder="Second experience" aria-label="Second experience" />
        </div>
        <div class="mt-3 flex flex-wrap gap-1.5">
          {#each EXPERIENCE_EXAMPLES as example (example)}
            <button class="dh-chip" type="button" onclick={() => fillExperience(example)}>{example}</button>
          {/each}
        </div>
      {:else if step === 10 && chosen}
        <p class="dh-muted mb-3 max-w-2xl text-sm">Pick two level 1 cards from {chosen.domains[0]} and {chosen.domains[1]}. Both may come from one domain.</p>
        <div class="dh-grid">
          {#each draft.cards as card (card.id)}
            <button class="dh-card text-left" class:on={card.taken} type="button" onclick={() => toggleCard(card.id)}>
              <p class="text-sm text-[var(--color-gold)]">{card.domain} · level 1</p>
              <p class="display mt-1">{card.name}</p>
              <p class="mt-2 max-h-40 overflow-auto whitespace-pre-wrap text-sm">{card.text}</p>
              <p class="mt-2 text-sm">{card.taken ? 'Chosen' : 'Available'}</p>
            </button>
          {/each}
        </div>
        <p class="mt-2 text-sm">{takenCards(draft).length} of 2 chosen</p>
      {:else if step === 11}
        <p class="dh-muted mb-3 max-w-2xl text-sm">Ask another player, or leave these until you are at the table. You can turn a question down.</p>
        <div class="grid max-w-2xl gap-3">
          {#each chosen?.connections ?? [] as question, index (question)}
            <label>{question}
              <textarea class="field mt-1 min-h-16" bind:value={draft.connections[index]} maxlength="500"></textarea>
            </label>
          {/each}
        </div>
      {:else if step === 12}
        <label class="mb-3 flex items-center gap-2 text-sm">
          <input type="checkbox" bind:checked={draft.place} disabled={!table.scene} />
          Place a token on {table.scene?.name ?? 'the scene, once there is one'}
        </label>
        <PcSheet character={previewCharacter(draft)} preview />
      {/if}
    </div>

    <footer class="dh-foot">
      <button class="btn quiet" type="button" disabled={step === 0} onclick={() => (step -= 1)}>Back</button>
      <p class="min-w-0 flex-1 text-sm text-[#e7a090]">{error}</p>
      {#if step < steps.length - 1}
        <button class="btn" type="button" disabled={!ready} onclick={() => (step += 1)}>Next</button>
      {:else}
        <button class="btn" type="button" disabled={busy || !complete} onclick={() => void finish()}>{busy ? 'Writing…' : 'Create character'}</button>
      {/if}
    </footer>
  </div>
</div>
