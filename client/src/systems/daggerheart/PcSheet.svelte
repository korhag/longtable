<script lang="ts">
  import type { Character } from '../../lib/net/bindings/Character.ts';
  import { send } from '../../lib/net/socket';
  import { TRAITS } from './catalog';
  import { adjust, writeCharacter } from './save';
  import './daggerheart.css';

  let { character, preview = false }: { character: Character; preview?: boolean } = $props();

  const number = (id: string, fallback = 0) => character.numbers[id] ?? fallback;
  const text = (id: string) => character.texts[id] ?? '';
  const signed = (value: number) => (value > 0 ? `+${value}` : `${value}`);
  const slots = (count: number) => Array.from({ length: Math.max(0, count) }, (_, index) => index);

  function setMarks(id: string, index: number, max: number) {
    if (preview) return;
    const current = number(id);
    const next = current === index + 1 ? index : index + 1;
    adjust(character.id, id, Math.min(max, Math.max(0, next)) - current);
  }

  function rename(value: string) {
    if (preview) return;
    const name = value.trim().slice(0, 24);
    if (name) writeCharacter(character.id, {}, {}, name);
  }
</script>

<article class="dh-sheet">
  <header class="dh-banner">
    <div>
      <p class="text-[11px] tracking-[0.22em] text-[var(--color-gold)] uppercase">{text('class') || 'Class'} · {text('domains') || 'Domains'}</p>
      {#if preview}
        <h2 class="display mt-1 text-2xl">{character.name}</h2>
      {:else}
        <input class="display mt-1 w-full bg-transparent text-2xl outline-none" value={character.name} maxlength="24" aria-label="Character name" onchange={(event) => rename(event.currentTarget.value)} />
      {/if}
      <p class="mt-1 text-[12px] text-[var(--color-parchment)]/70">
        {text('pronouns') || 'Pronouns'} · {text('ancestry') || 'Ancestry'} {text('community') || 'Community'} · {text('subclass') || 'Subclass'}
      </p>
    </div>
    <div class="dh-shield">
      <b>{number('level', 1)}</b>
      <span>Level</span>
    </div>
  </header>

  <div class="flex flex-wrap gap-2">
    <div class="dh-shield">
      <b>{number('evasion', 10)}</b>
      <span>Evasion</span>
    </div>
    <div class="dh-shield">
      <b>{number('armorscore')}</b>
      <span>Armor</span>
    </div>
    {#each TRAITS as trait (trait.id)}
      <div class="dh-shield" title={trait.verbs}>
        <b>{signed(number(trait.id))}</b>
        <span>{trait.name}</span>
      </div>
    {/each}
  </div>

  <div class="dh-columns">
    <div class="flex flex-col gap-2">
      <section class="dh-block">
        <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Damage & health</p>
        <div class="mt-2 grid grid-cols-3 gap-2 text-center text-[12px]">
          <div>
            <p class="text-[var(--color-parchment)]/55">Minor</p>
            <p class="display text-xl">{number('major', 7)}</p>
            <p class="text-[10px]">Mark 1 HP</p>
          </div>
          <div>
            <p class="text-[var(--color-parchment)]/55">Major</p>
            <p class="display text-xl">{number('severe', 14)}</p>
            <p class="text-[10px]">Mark 2 HP</p>
          </div>
          <div>
            <p class="text-[var(--color-parchment)]/55">Severe</p>
            <p class="display text-xl">+</p>
            <p class="text-[10px]">Mark 3 HP</p>
          </div>
        </div>
        <p class="mt-3 text-[11px] tracking-[0.14em] text-[var(--color-gold)] uppercase">HP</p>
        <div class="dh-boxes mt-1">
          {#each slots(Math.max(1, number('hpmax', 6))) as index (index)}
            <button class:on={index < number('hp')} type="button" aria-label={`Hit point ${index + 1}`} onclick={() => setMarks('hp', index, number('hpmax', 6))}></button>
          {/each}
        </div>
        <p class="mt-3 text-[11px] tracking-[0.14em] text-[var(--color-gold)] uppercase">Stress</p>
        <div class="dh-boxes mt-1">
          {#each slots(Math.max(1, number('stressmax', 6))) as index (index)}
            <button class:on={index < number('stress')} type="button" aria-label={`Stress ${index + 1}`} onclick={() => setMarks('stress', index, number('stressmax', 6))}></button>
          {/each}
        </div>
        {#if !preview}
          <form class="mt-3 flex gap-1" onsubmit={(event) => {
            event.preventDefault();
            const form = event.currentTarget;
            const amount = Number(new FormData(form).get('amount'));
            if (amount > 0) send({ type: 'applyDamage', characterId: character.id, fieldId: 'thresholds', amount });
            form.reset();
          }}>
            <input class="field" name="amount" type="number" min="1" max="99" placeholder="Damage" aria-label="Damage amount" />
            <button class="btn" type="submit">Apply</button>
          </form>
        {/if}
      </section>

      <section class="dh-block">
        <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Hope</p>
        <div class="dh-boxes dh-pips mt-1">
          {#each slots(6) as index (index)}
            <button class:on={index < number('hope')} type="button" aria-label={`Hope ${index + 1}`} onclick={() => setMarks('hope', index, 6)}></button>
          {/each}
        </div>
        <p class="mt-2 text-[12px] text-[var(--color-parchment)]/60">You start with 2. A Hope roll can raise it, up to 6.</p>
      </section>

      <section class="dh-block">
        <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Experiences</p>
        <p class="mt-1 whitespace-pre-wrap text-sm">{text('experiences') || 'Two experiences, each +2.'}</p>
      </section>

      <section class="dh-block">
        <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Gold</p>
        <p class="display mt-1 text-lg">{number('gold', 1)} handful{number('gold', 1) === 1 ? '' : 's'}</p>
      </section>

      <section class="dh-block">
        <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Class feature</p>
        {#each text('feature').split('\n').filter(Boolean) as feature (feature)}
          <p class="mt-2 text-sm"><b>{feature}</b></p>
          <p class="text-[12px] text-[var(--color-parchment)]/55">The rules for this feature come later.</p>
        {/each}
      </section>
    </div>

    <div class="flex flex-col gap-2">
      <section class="dh-block">
        <div class="flex items-baseline justify-between">
          <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Active weapons</p>
          <p class="text-[12px]">Proficiency {number('proficiency', 1)}</p>
        </div>
        <p class="mt-2 text-sm">{text('primary') || 'No primary weapon'}</p>
        <p class="mt-1 text-[12px] text-[var(--color-parchment)]/60">{text('damage') ? `Damage ${text('damage')}` : ''}</p>
        {#if text('secondary')}
          <p class="mt-3 text-[11px] tracking-[0.14em] text-[var(--color-gold)] uppercase">Secondary</p>
          <p class="mt-1 text-sm">{text('secondary')}</p>
        {/if}
      </section>

      <section class="dh-block">
        <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Active armor</p>
        <p class="mt-2 text-sm">{text('armorname') || 'No armor'}</p>
        <p class="mt-2 text-[11px] tracking-[0.14em] text-[var(--color-gold)] uppercase">Armor slots</p>
        <div class="dh-boxes dh-pips mt-1">
          {#each slots(number('armorscore')) as index (index)}
            <button class:on={index < number('armor')} type="button" aria-label={`Armor slot ${index + 1}`} onclick={() => setMarks('armor', index, number('armorscore'))}></button>
          {/each}
        </div>
        <p class="mt-2 text-[12px] text-[var(--color-parchment)]/60">Mark one slot to drop the damage by one step.</p>
      </section>

      <section class="dh-block">
        <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Inventory</p>
        <p class="mt-1 whitespace-pre-wrap text-sm">{text('inventory') || 'Empty'}</p>
      </section>

      <section class="dh-block">
        <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Domain cards</p>
        <p class="mt-1 whitespace-pre-wrap text-sm">{text('cards') || 'No cards yet.'}</p>
        <p class="mt-1 text-[12px] text-[var(--color-parchment)]/55">Level 1 cards. Their abilities come later.</p>
      </section>

      {#if text('description') || text('background') || text('connections')}
        <section class="dh-block">
          <p class="text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Story</p>
          {#if text('description')}<p class="mt-1 whitespace-pre-wrap text-sm">{text('description')}</p>{/if}
          {#if text('background')}<p class="mt-2 whitespace-pre-wrap text-[13px] text-[var(--color-parchment)]/75">{text('background')}</p>{/if}
          {#if text('connections')}<p class="mt-2 whitespace-pre-wrap text-[13px] text-[var(--color-parchment)]/75">{text('connections')}</p>{/if}
        </section>
      {/if}
    </div>
  </div>
</article>
