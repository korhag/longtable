<script lang="ts">
  import { portal, type Anchor } from '../ui/portal';

  let { name, anchor, onkick, onclose }: {
    name: string;
    anchor: Anchor;
    onkick: () => void;
    onclose: () => void;
  } = $props();

  // The portraits sit in the top bar, so the menu opens downward.
  const box = $derived(
    `position:fixed;left:${Math.max(8, Math.min(anchor.left, window.innerWidth - 188))}px;top:${anchor.bottom + 8}px;width:180px;z-index:calc(var(--z-menu) + 1)`,
  );
</script>

<div use:portal>
  <button class="menu-shade" type="button" aria-label="Close seat menu" onclick={onclose} oncontextmenu={(event) => { event.preventDefault(); onclose(); }}></button>
  <div class="glass rounded-2xl p-2 text-left" style={box}>
    <p class="truncate px-2 py-1 text-xs text-[var(--color-parchment)]/70">{name}</p>
    <button class="w-full rounded-lg px-2 py-1 text-left text-xs text-[#e7a090] hover:bg-white/5" type="button" onclick={onkick}>Kick</button>
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
