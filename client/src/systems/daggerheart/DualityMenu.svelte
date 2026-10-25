<script lang="ts">
  import { DEFAULT_DUALITY, type DualityConfig, type DualityDie, type DualityEdge } from './duality';

  let { config, onchange, onclose }: {
    config: DualityConfig;
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

<button class="fixed inset-0 z-20 cursor-default" type="button" aria-label="Close duality options" onclick={onclose}></button>
<div class="glass absolute bottom-full left-0 z-30 mb-2 w-64 rounded-2xl p-3 text-left">
  <p class="text-[10px] tracking-[0.18em] text-[var(--color-gold)] uppercase">Duality</p>
  <div class="mt-2 flex items-center justify-between gap-2">
    <span class="text-xs">Hope</span>
    <div class="flex gap-1">
      {#each dice as sides (sides)}
        <button class="die-choice" class:on={config.hope === sides} type="button" onclick={() => set({ hope: sides })}>d{sides}</button>
      {/each}
    </div>
  </div>
  <div class="mt-2 flex items-center justify-between gap-2">
    <span class="text-xs">Fear</span>
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
  <button class="mt-3 text-xs text-[var(--color-parchment)]/70" type="button" onclick={() => onchange({ ...DEFAULT_DUALITY })}>Reset</button>
</div>

<style>
  .die-choice {
    border-radius: 0.5rem;
    border: 1px solid rgba(212, 166, 86, 0.3);
    padding: 0.25rem 0.55rem;
    font-size: 0.75rem;
  }
  .die-choice.on {
    background: rgba(255, 255, 255, 0.14);
    color: var(--color-gold-bright);
  }
</style>
