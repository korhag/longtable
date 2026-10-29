<script lang="ts">
  import { applyModifiers } from '../../lib/dice/rollText';
  import { rollSetup } from '../../lib/dice/rollSetup.svelte';
  import { send } from '../../lib/net/socket';
  import { DEFAULT_DUALITY, dualityCaption, dualityFormula, type DualityConfig } from './duality';
  import DualityMenu from './DualityMenu.svelte';

  const STORAGE_KEY = 'game-table.duality';

  let config = $state<DualityConfig>(loadDuality());
  let menu = $state(false);
  let anchor = $state({ left: 0, top: 0, right: 0, bottom: 0 });

  function roll() {
    send({
      type: 'roll',
      formula: applyModifiers(dualityFormula(config), rollSetup.plus, rollSetup.minus),
      visibility: rollSetup.visibility,
      label: 'Duality',
    });
  }

  function save(next: DualityConfig) {
    config = next;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  }

  function loadDuality(): DualityConfig {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return { ...DEFAULT_DUALITY };
      const parsed = JSON.parse(raw) as Partial<DualityConfig> & { modifier?: number };
      const hope = parsed.hope === 20 ? 20 : 12;
      const fear = parsed.fear === 20 ? 20 : 12;
      const edge = parsed.edge === 'advantage' || parsed.edge === 'disadvantage' ? parsed.edge : 'none';
      return { hope, fear, edge };
    } catch {
      return { ...DEFAULT_DUALITY };
    }
  }
</script>

<div class="relative">
  <button
    class="duality-pip"
    type="button"
    title="Left click rolls. Right click changes Hope, Fear, and advantage."
    onclick={roll}
    oncontextmenu={(event) => {
      event.preventDefault();
      if (menu) {
        menu = false;
        return;
      }
      const box = event.currentTarget.getBoundingClientRect();
      anchor = { left: box.left, top: box.top, right: box.right, bottom: box.bottom };
      menu = true;
    }}
  >
    <span class="swatch" style="background:#d4a656"></span>
    <span class="swatch fear" style="background:#3c2158"></span>
    {dualityCaption(config)}
  </button>
  {#if menu}
    <DualityMenu {config} {anchor} onchange={save} onclose={() => (menu = false)} />
  {/if}
</div>

<style>
  .duality-pip {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    height: 2.7rem;
    padding: 0 0.55rem;
    border-radius: 0.7rem;
    border: 1px solid rgba(255, 236, 210, 0.2);
    background: #1c140e;
    font-family: var(--font-display);
    font-size: 0.875rem;
    color: #f6efe4;
    white-space: nowrap;
  }
  .swatch {
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 999px;
  }
  .swatch.fear {
    box-shadow: inset 0 0 0 1px rgba(244, 236, 223, 0.35);
  }
</style>
