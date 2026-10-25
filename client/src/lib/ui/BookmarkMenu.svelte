<script lang="ts">
  import type { DiceBookmark } from '../dice/bookmarks';

  let { bookmarks, onpick, onremove, onclose }: {
    bookmarks: DiceBookmark[];
    onpick: (bookmark: DiceBookmark) => void;
    onremove: (label: string) => void;
    onclose: () => void;
  } = $props();
</script>

<button class="fixed inset-0 z-20 cursor-default" type="button" aria-label="Close saved rolls" onclick={onclose}></button>
<div class="glass absolute right-0 bottom-full z-30 mb-2 w-56 rounded-2xl p-2 text-left">
  {#if bookmarks.length === 0}
    <p class="px-2 py-1 text-xs text-[var(--color-parchment)]/60">Nothing saved yet.</p>
  {:else}
    <ul class="flex max-h-52 flex-col gap-0.5 overflow-auto">
      {#each bookmarks as bookmark (bookmark.label)}
        <li class="flex items-center gap-1">
          <button class="min-w-0 flex-1 truncate rounded-lg px-2 py-1 text-left text-xs" type="button" onclick={() => onpick(bookmark)}>
            {bookmark.label}
          </button>
          <button class="px-1.5 text-xs text-[#e7a090]" type="button" aria-label={`Delete ${bookmark.label}`} onclick={() => onremove(bookmark.label)}>✕</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>
