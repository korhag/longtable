<script lang="ts">
  import type { DiceBookmark } from '../dice/bookmarks';
  import { menuBox, portal, type Anchor } from './portal';

  let { bookmarks, anchor, onpick, onremove, onclose }: {
    bookmarks: DiceBookmark[];
    anchor: Anchor;
    onpick: (bookmark: DiceBookmark) => void;
    onremove: (label: string) => void;
    onclose: () => void;
  } = $props();
</script>

<div use:portal>
  <button class="menu-shade" type="button" aria-label="Close saved rolls" onclick={onclose} oncontextmenu={(event) => { event.preventDefault(); onclose(); }}></button>
  <div class="glass overflow-auto rounded-2xl p-2 text-left" style={menuBox(anchor, 224)}>
    {#if bookmarks.length === 0}
      <p class="px-2 py-1 text-sm text-[var(--color-parchment)]/60">Nothing saved yet.</p>
    {:else}
      <ul class="flex flex-col gap-0.5">
        {#each bookmarks as bookmark (bookmark.label)}
          <li class="flex items-center gap-1">
            <button class="min-w-0 flex-1 truncate rounded-lg px-2 py-1 text-left text-sm" type="button" onclick={() => onpick(bookmark)}>
              {bookmark.label}
            </button>
            <button class="px-1.5 text-sm text-[#e7a090]" type="button" aria-label={`Delete ${bookmark.label}`} onclick={() => onremove(bookmark.label)}>✕</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .menu-shade {
    position: fixed;
    inset: 0;
    z-index: var(--z-menu);
    cursor: default;
    background: transparent;
  }
</style>
