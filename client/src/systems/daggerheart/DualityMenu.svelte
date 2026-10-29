<script lang="ts">
  import { menuBox, portal, type Anchor } from '../../lib/ui/portal';
  import { DEFAULT_DUALITY, type DualityConfig, type DualityDie, type DualityEdge } from './duality';

  let { config, anchor, onchange, onclose }: {
    config: DualityConfig;
    anchor: Anchor;
    onchange: (next: DualityConfig) => void;
    onclose: () => void;
  } = $props();

  const dice: DualityDie[] = [12, 20];

  function set(patch: Partial<DualityConfig>) {
    onchange({ ...config, ...patch });
  }

  function chooseEdge(edge: DualityEdge) {
    set({ edge: config.edge === edge ? 'none' : edge });
  }
</script>

<div use:portal>
  <button class="menu-shade" type="button" aria-label="Close duality options" onclick={onclose} oncontextmenu={(event) => { event.preventDefault(); onclose(); }}></button>
  <div class="glass overflow-auto rounded-2xl p-3 text-left" style={menuBox(anchor, 256)}>
  <p class="text-sm tracking-[0.18em] text-[var(--color-gold)] uppercase">Duality</p>
  <div class="mt-2 flex items-center justify-between gap-2">
    <span class="text-sm">Hope</span>
    <div class="flex gap-1">
      {#each dice as sides (sides)}
        <button class="die-choice" class:on={config.hope === sides} type="button" onclick={() => set({ hope: sides })}>d{sides}</button>
      {/each}
    </div>
  </div>
  <div class="mt-2 flex items-center justify-between gap-2">
    <span class="text-sm">Fear</span>
    <div class="flex gap-1">
      {#each dice as sides (sides)}
        <button class="die-choice" class:on={config.fear === sides} type="button" onclick={() => set({ fear: sides })}>d{sides}</button>
      {/each}
    </div>
  </div>
  <div class="mt-3 flex gap-1">
    <button class="die-choice flex-1" class:on={config.edge === 'advantage'} type="button" onclick={() => chooseEdge('advantage')}>+1d6</button>
    <button class="die-choice flex-1" class:on={config.edge === 'disadvantage'} type="button" onclick={() => chooseEdge('disadvantage')}>−1d6</button>
  </div>
  <button class="mt-3 text-sm text-[var(--color-parchment)]/70" type="button" onclick={() => onchange({ ...DEFAULT_DUALITY })}>Reset</button>
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
  .die-choice {
    border-radius: 0.5rem;
    border: 1px solid rgba(212, 166, 86, 0.3);
    padding: 0.25rem 0.55rem;
    font-size: 0.875rem;
  }
  .die-choice.on {
    background: rgba(255, 255, 255, 0.14);
    color: var(--color-gold-bright);
  }
</style>
