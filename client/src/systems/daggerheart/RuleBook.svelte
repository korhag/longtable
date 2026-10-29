<script lang="ts">
  import { CLASSES, DOMAIN_CARDS, RULES, ruleHtml } from './catalog';

  let { close }: { close: () => void } = $props();

  let chapter = $state<'rules' | 'classes' | 'cards'>('rules');
  let picked = $state(RULES[0]?.name ?? '');
  let domain = $state('Arcana');
  let query = $state('');

  const domains = $derived([...new Set(DOMAIN_CARDS.map((card) => card.domain))]);
  const section = $derived(RULES.find((item) => item.name === picked) ?? RULES[0]);
  const classEntry = $derived(CLASSES.find((item) => item.name === picked) ?? CLASSES[0]);
  const cards = $derived(
    DOMAIN_CARDS.filter((card) => card.domain === domain && card.name.toLowerCase().includes(query.trim().toLowerCase())),
  );

  function openChapter(next: 'rules' | 'classes' | 'cards') {
    chapter = next;
    if (next === 'rules') picked = RULES[0]?.name ?? '';
    if (next === 'classes') picked = CLASSES[0]?.name ?? '';
  }
</script>

<div class="dh-overlay" style="grid-template-columns: 16rem 1fr">
  <div class="dh-rail" style="border-right: 1px solid rgba(212,166,86,.22)">
    <div class="flex items-center justify-between px-1">
      <p class="display text-sm text-[var(--color-gold-bright)]">Daggerheart</p>
      <button class="btn quiet px-2" type="button" onclick={close}>Close</button>
    </div>
    <div class="mt-3 flex flex-wrap gap-1">
      <button class="dh-chip" class:on={chapter === 'rules'} type="button" onclick={() => openChapter('rules')}>Rules</button>
      <button class="dh-chip" class:on={chapter === 'classes'} type="button" onclick={() => openChapter('classes')}>Classes</button>
      <button class="dh-chip" class:on={chapter === 'cards'} type="button" onclick={() => openChapter('cards')}>Cards</button>
    </div>
    <div class="mt-3 flex flex-col gap-1 overflow-auto pr-1">
      {#if chapter === 'rules'}
        {#each RULES as item (item.name)}
          <button class="rounded px-2 py-1 text-left text-sm hover:bg-white/10 {picked === item.name ? 'text-[var(--color-gold-bright)]' : ''}" type="button" onclick={() => (picked = item.name)}>{item.name}</button>
        {/each}
      {:else if chapter === 'classes'}
        {#each CLASSES as item (item.id)}
          <button class="rounded px-2 py-1 text-left text-sm hover:bg-white/10" type="button" onclick={() => (picked = item.name)}>{item.name}</button>
        {/each}
      {:else}
        {#each domains as name (name)}
          <button class="dh-chip" class:on={domain === name} type="button" onclick={() => (domain = name)}>{name}</button>
        {/each}
        <input class="field mt-2" bind:value={query} placeholder="Search this domain" aria-label="Search domain cards" />
      {/if}
    </div>
  </div>
  <div class="dh-body">
    {#if chapter === 'rules' && section}
      <h2 class="display text-2xl">{section.name}</h2>
      <div class="rule-copy mt-4 max-w-3xl text-sm leading-relaxed">{@html ruleHtml(section.text)}</div>
    {:else if chapter === 'classes' && classEntry}
      <h2 class="display text-2xl">{classEntry.name}</h2>
      <p class="mt-1 text-sm text-[var(--color-gold)]">{classEntry.domains[0]} & {classEntry.domains[1]} · Evasion {classEntry.evasion} · HP {classEntry.hp} · Spellcast {classEntry.spellcastLabel}</p>
      <p class="mt-3 max-w-3xl text-sm leading-relaxed">{classEntry.description}</p>
      <div class="mt-4 max-w-3xl whitespace-pre-wrap text-sm leading-relaxed">{classEntry.features.join('\n\n')}</div>
      <h3 class="display mt-4 text-lg">{classEntry.hope.split('\n')[0]}</h3>
      <p class="mt-1 max-w-3xl whitespace-pre-wrap text-sm">{classEntry.hope.split('\n').slice(1).join('\n')}</p>
      {#each classEntry.subclasses as item (item.id)}
        <h3 class="display mt-5 text-lg">{item.name}</h3>
        <p class="mt-1 text-sm text-[var(--color-gold)]">{item.spellcast ? `Spellcast ${item.spellcast}` : 'No spellcast trait'}</p>
        <p class="mt-2 text-sm">{item.pitch}</p>
        <p class="mt-3 text-sm tracking-[0.14em] text-[var(--color-gold)] uppercase">Foundation</p>
        <p class="mt-1 whitespace-pre-wrap text-sm">{item.foundation}</p>
        <p class="mt-3 text-sm tracking-[0.14em] text-[var(--color-gold)] uppercase">Specialization</p>
        <p class="mt-1 whitespace-pre-wrap text-sm">{item.specialization}</p>
        <p class="mt-3 text-sm tracking-[0.14em] text-[var(--color-gold)] uppercase">Mastery</p>
        <p class="mt-1 whitespace-pre-wrap text-sm">{item.mastery}</p>
      {/each}
    {:else}
      <h2 class="display text-2xl">{domain}</h2>
      <div class="mt-4 grid max-w-5xl gap-3">
        {#each cards as card (card.id)}
          <article class="dh-card">
            <p class="text-sm text-[var(--color-gold)]">Level {card.level} · {card.kind} · Recall {card.recall}</p>
            <p class="display mt-1">{card.name}</p>
            <p class="mt-2 whitespace-pre-wrap text-sm">{card.text}</p>
          </article>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .rule-copy :global(h2) {
    margin-top: 1.25rem;
    font-family: var(--font-display, inherit);
    font-size: 1.25rem;
  }
  .rule-copy :global(h3) {
    margin-top: 1rem;
    font-size: 1rem;
  }
  .rule-copy :global(p) {
    margin-top: 0.75rem;
  }
  .rule-copy :global(.rule-table) {
    margin-top: 0.75rem;
    overflow-x: auto;
  }
  .rule-copy :global(table) {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }
  .rule-copy :global(th),
  .rule-copy :global(td) {
    padding: 0.45rem 0.6rem;
    border-bottom: 1px solid rgba(212, 166, 86, 0.28);
    text-align: left;
    vertical-align: top;
  }
  .rule-copy :global(th) {
    color: var(--color-gold);
    font-weight: 600;
  }
</style>
