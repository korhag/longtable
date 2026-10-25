<script lang="ts">
  import { bookmarkMatches, loadBookmarks, removeBookmark, saveBookmarks, upsertBookmark, type DiceBookmark } from '../dice/bookmarks';
  import { sound } from '../dice/sound.svelte';
  import { dieLook, STANDARD_SIDES } from '../dice/palette';
  import { rollSetup } from '../dice/rollSetup.svelte';
  import { applyModifiers } from '../dice/rollText';
  import { send } from '../net/socket';
  import { systems } from '../systems/store.svelte';
  import { extraFor } from '../../systems/index';
  import { table } from '../table.svelte';
  import BookmarkMenu from './BookmarkMenu.svelte';
  import DiePip from './DiePip.svelte';
  import ModifierBox from './ModifierBox.svelte';

  const faces = STANDARD_SIDES.map((sides) => ({ sides, ...dieLook(sides) }));
  const Extra = $derived(extraFor(systems.active?.id ?? null));

  let formula = $state('1d20');
  let label = $state('');
  let savedOpen = $state(false);
  let bookmarks = $state<DiceBookmark[]>(loadBookmarks());

  const modes = $derived(
    table.role === 'gm'
      ? [
          { id: 'public' as const, name: 'Everyone' },
          { id: 'toGm' as const, name: 'To the GM' },
          { id: 'gmSecret' as const, name: 'Secret' },
        ]
      : [
          { id: 'public' as const, name: 'Everyone' },
          { id: 'toGm' as const, name: 'To the GM' },
        ],
  );

  const starred = $derived(bookmarks.some((item) => bookmarkMatches(item, { label, formula, plus: rollSetup.plus, minus: rollSetup.minus })));

  function addDie(sides: number) {
    const trimmed = formula.trim();
    formula = trimmed ? `${trimmed}+1d${sides}` : `1d${sides}`;
  }

  function roll() {
    const text = formula.trim();
    if (!text) return;
    send({ type: 'roll', formula: applyModifiers(text, rollSetup.plus, rollSetup.minus), visibility: rollSetup.visibility, label: label.trim() || null });
  }

  function clear() {
    formula = '';
    rollSetup.plus = 0;
    rollSetup.minus = 0;
  }

  function remember() {
    const name = label.trim();
    if (!name) return;
    bookmarks = upsertBookmark(bookmarks, { label: name, formula, plus: rollSetup.plus, minus: rollSetup.minus });
    saveBookmarks(bookmarks);
  }

  function pick(bookmark: DiceBookmark) {
    label = bookmark.label;
    formula = bookmark.formula;
    rollSetup.plus = bookmark.plus;
    rollSetup.minus = bookmark.minus;
    savedOpen = false;
  }

  function forget(name: string) {
    bookmarks = removeBookmark(bookmarks, name);
    saveBookmarks(bookmarks);
  }
</script>

<div class="flex flex-col gap-2">
  <div class="flex flex-wrap items-center gap-1.5">
    {#each faces as face (face.sides)}
      <DiePip sides={face.sides} body={face.body} ink={face.ink} onclick={() => addDie(face.sides)} />
    {/each}
    {#if Extra}
      <Extra />
    {/if}
    <div class="relative ml-auto">
      <button class="btn quiet px-2 py-1 text-xs" type="button" onclick={() => (savedOpen = !savedOpen)}>Saved</button>
      {#if savedOpen}
        <BookmarkMenu {bookmarks} onpick={pick} onremove={forget} onclose={() => (savedOpen = false)} />
      {/if}
    </div>
  </div>

  <div class="flex items-center gap-1.5">
    <input class="field min-w-0 flex-1" bind:value={label} maxlength="40" placeholder="Label" aria-label="Roll label" />
    <button
      class="star"
      class:on={starred}
      type="button"
      aria-label="Save this roll"
      disabled={!label.trim()}
      onclick={remember}
    >★</button>
  </div>

  <input class="field" bind:value={formula} aria-label="Dice formula" onkeydown={(event) => { if (event.key === 'Enter') roll(); }} />

  <div class="flex items-center gap-1.5">
    <ModifierBox sign="+" value={rollSetup.plus} onchange={(next) => (rollSetup.plus = next)} />
    <ModifierBox sign="−" value={rollSetup.minus} onchange={(next) => (rollSetup.minus = next)} />
  </div>

  <div class="flex flex-col overflow-hidden rounded-lg border border-[rgba(212,166,86,0.3)]">
    {#each modes as mode (mode.id)}
      <button
        class={`px-3 py-2 text-left text-xs ${rollSetup.visibility === mode.id ? 'bg-white/15' : ''}`}
        type="button"
        onclick={() => (rollSetup.visibility = mode.id)}
      >{mode.name}</button>
    {/each}
  </div>
  <div class="flex items-center gap-1.5">
    <button class="btn quiet px-2 text-xs" type="button" onclick={() => sound.set(!sound.muted)}>{sound.muted ? 'Sound off' : 'Sound on'}</button>
    <button class="btn quiet px-2 text-xs" type="button" onclick={clear}>Clear</button>
    <button class="btn display ml-auto px-4 text-sm" type="button" onclick={roll}>Roll</button>
  </div>
</div>

<style>
  .star {
    width: 2rem;
    height: 2rem;
    border-radius: 0.6rem;
    color: rgba(246, 239, 228, 0.45);
    font-size: 1.1rem;
  }
  .star.on {
    color: var(--color-gold-bright);
  }
  .star:disabled {
    opacity: 0.35;
  }
</style>
