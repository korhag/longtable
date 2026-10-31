<script lang="ts">
  import type { EnvironmentLighting } from '../net/bindings/EnvironmentLighting';
  import { DAYLIGHT, ENVIRONMENT_PRESETS } from '../lightingPresets';
  let { environment, onchange }: { environment?: EnvironmentLighting | null; onchange: (value: EnvironmentLighting, commit: boolean) => void } = $props();
  const current = $derived(environment ?? DAYLIGHT);
  let brightness = $state(1);
  $effect(() => { brightness = current.brightness; });
  function changeBrightness(value: number, commit: boolean) {
    brightness = value;
    onchange({ ...current, brightness: value }, commit);
  }
</script>
<div class="environment-controls">
  <div class="environment-presets" aria-label="Time of day">
    {#each ENVIRONMENT_PRESETS as preset}
      <button class="btn quiet" class:active={current.preset === preset.value.preset} aria-pressed={current.preset === preset.value.preset} type="button" onclick={() => onchange({ ...preset.value }, true)}>{preset.name}</button>
    {/each}
  </div>
  <label>
    <span>Ambient brightness <output>{Math.round(brightness * 100)}%</output></span>
    <input type="range" aria-label="Ambient brightness" min="0.05" max="1" step="0.01" value={brightness} oninput={e => changeBrightness(Number(e.currentTarget.value), false)} onchange={e => changeBrightness(Number(e.currentTarget.value), true)} />
  </label>
</div>
<style>
  .environment-controls { display:grid; gap:12px; min-width:0; }
  .environment-presets { display:grid; grid-template-columns:repeat(3,minmax(0,1fr)); gap:6px; }
  .environment-presets button { padding-inline:8px; }
  label { display:grid; gap:6px; font-size:12px; }
  label span { display:flex; justify-content:space-between; gap:16px; }
  output { color:var(--color-gold); font-variant-numeric:tabular-nums; }
  input { width:100%; accent-color:var(--color-gold); }
</style>
