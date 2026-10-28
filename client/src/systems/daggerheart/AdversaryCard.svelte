<script lang="ts">
  import type { Character } from '../../lib/net/bindings/Character.ts';
  import { send } from '../../lib/net/socket';
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

  function attack() {
    if (preview) return;
    send({ type: 'rollAction', characterId: character.id, actionId: 'attackroll', visibility: 'public', plus: 0, minus: 0 });
  }
</script>

<article class="dh-adv">
  <h2>{character.name}</h2>
  <p class="mt-1 text-[12px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Tier {number('tier', 1)} {text('advtype') || 'Standard'}</p>
  {#if text('description')}
    <p class="mt-2 text-sm italic text-[var(--color-parchment)]/80">{text('description')}</p>
  {/if}
  {#if text('motives')}
    <p class="mt-2 text-sm"><b>Motives & tactics</b> {text('motives')}</p>
  {/if}

  <div class="dh-stat">
    <span>Difficulty: <b>{number('difficulty', 12)}</b></span>
    <span>Thresholds: <b>{number('major', 8)}/{number('severe', 14)}</b></span>
    <span>HP: <b>{number('hpmax', 5)}</b></span>
    <span>Stress: <b>{number('stressmax', 3)}</b></span>
  </div>

  <p class="text-sm">
    ATK {signed(number('attack', 1))} · {text('attackname') || 'Attack'}: {text('attackrange') || 'Melee'} · {text('atkdamage') || '1d8'} phy
  </p>
  {#if text('advexp')}
    <p class="mt-1 text-sm"><b>Experience:</b> {text('advexp')}</p>
  {/if}

  <div class="mt-3 flex flex-wrap items-center gap-3">
    <div>
      <p class="text-[10px] tracking-[0.14em] text-[var(--color-gold)] uppercase">HP marked</p>
      <div class="dh-boxes mt-1">
        {#each slots(Math.max(1, number('hpmax', 5))) as index (index)}
          <button class:on={index < number('hp')} type="button" aria-label={`Adversary hit point ${index + 1}`} onclick={() => setMarks('hp', index, number('hpmax', 5))}></button>
        {/each}
      </div>
    </div>
    <div>
      <p class="text-[10px] tracking-[0.14em] text-[var(--color-gold)] uppercase">Stress</p>
      <div class="dh-boxes mt-1">
        {#each slots(number('stressmax', 3)) as index (index)}
          <button class:on={index < number('stress')} type="button" aria-label={`Adversary stress ${index + 1}`} onclick={() => setMarks('stress', index, number('stressmax', 3))}></button>
        {/each}
      </div>
    </div>
  </div>

  {#if !preview}
    <div class="mt-3 flex gap-1">
      <button class="btn" type="button" onclick={attack}>Attack</button>
      <form class="flex flex-1 gap-1" onsubmit={(event) => {
        event.preventDefault();
        const form = event.currentTarget;
        const amount = Number(new FormData(form).get('amount'));
        if (amount > 0) send({ type: 'applyDamage', characterId: character.id, fieldId: 'thresholds', amount });
        form.reset();
      }}>
        <input class="field" name="amount" type="number" min="1" max="99" placeholder="Damage" aria-label="Damage amount" />
        <button class="btn quiet" type="submit">Apply</button>
      </form>
    </div>
  {/if}

  <p class="mt-4 text-[11px] tracking-[0.16em] text-[var(--color-gold)] uppercase">Features</p>
  <div class="mt-1 whitespace-pre-wrap text-sm leading-relaxed">{text('features') || 'No features yet.'}</div>

  {#if !preview}
    <label class="mt-3 block text-[11px] text-[var(--color-parchment)]/55">Features
      <textarea class="field mt-1 min-h-20" value={text('features')} onchange={(event) => writeCharacter(character.id, {}, { features: event.currentTarget.value })}></textarea>
    </label>
  {/if}
</article>
