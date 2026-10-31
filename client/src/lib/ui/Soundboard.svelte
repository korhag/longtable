<script lang="ts">
  import { onDestroy } from 'svelte';
  import { sound } from '../dice/sound.svelte';
  import { COMBAT_ACTIONS, COMBAT_VOICES, playCombat, safeVolume, setCombatVolume, stopCombat, type CombatAction, type CombatVoice } from '../audio/combat';
  const voiceKey = 'longtable.combat-voice';
  const volumeKey = 'longtable.combat-volume';
  function saved(key: string) { try { return localStorage.getItem(key); } catch { return null; } }
  function keep(key: string, value: string) { try { localStorage.setItem(key, value); } catch { /* Playback still works when storage is unavailable. */ } }
  let voice = $state<CombatVoice>(saved(voiceKey) === 'male' ? 'male' : 'female');
  let volume = $state(safeVolume(Number(saved(volumeKey) ?? 0.65)));
  let playing = $state<CombatAction | null>(null);
  let note = $state('');
  let serial = 0;
  const halt = () => { serial += 1; stopCombat(); playing = null; };
  onDestroy(halt);
  $effect(() => { if (sound.muted) halt(); });
  async function play(action: CombatAction) {
    const request = ++serial;
    playing = action; note = '';
    try {
      const started = await playCombat(voice, action, { volume, muted: sound.muted, onEnded: () => { if (request === serial) playing = null; } });
      if (!started && request === serial) playing = null;
    } catch {
      if (request === serial) { playing = null; note = 'Sound could not play. Try again after enabling browser audio.'; }
    }
  }
  function choose(next: CombatVoice) { halt(); voice = next; keep(voiceKey, next); }
  function changeVolume(value: number) { volume = safeVolume(value); setCombatVolume(volume); keep(volumeKey, String(volume)); }
</script>

<div class="soundboard">
  <div class="voice-heading"><strong>Combat voices</strong><span>Local playback</span></div>
  <div class="voice-options" aria-label="Combat voice">
    {#each COMBAT_VOICES as option}
      <button class="btn quiet" class:active={voice === option} aria-pressed={voice === option} type="button" onclick={() => choose(option)}>{option === 'female' ? 'Female' : 'Male'}</button>
    {/each}
  </div>
  <div class="combat-actions">
    {#each COMBAT_ACTIONS as action}
      <button class="btn quiet" class:active={playing === action.id} type="button" disabled={sound.muted || volume === 0} aria-label={`Play ${voice} ${action.label.toLowerCase()}`} onclick={() => void play(action.id)}><span aria-hidden="true">▶</span> {action.label}</button>
    {/each}
  </div>
  <label class="combat-volume"><span>Voice volume <output>{Math.round(volume * 100)}%</output></span><input aria-label="Voice volume" type="range" min="0" max="1" step="0.05" value={volume} oninput={e => changeVolume(Number(e.currentTarget.value))} /></label>
  <div class="voice-options">
    <button class="btn quiet" type="button" aria-pressed={sound.muted} onclick={() => sound.set(!sound.muted)}>{sound.muted ? 'Sound off' : 'Sound on'}</button>
    <button class="btn quiet" type="button" disabled={!playing} onclick={halt}>Stop</button>
  </div>
  {#if note}<p role="status">{note}</p>{/if}
  <p>Choose a voice and play a cue on this computer. Sound on/off also controls dice.</p>
  <a href="/audio/combat/ATTRIBUTION.md" target="_blank" rel="noreferrer">Audio credits · CC BY-SA 3.0</a>
</div>

<style>
  .soundboard { display:grid; gap:10px; width:264px; max-width:100%; }
  .voice-heading { display:flex; justify-content:space-between; align-items:center; gap:8px; }
  strong { color:var(--color-gold); font-size:13px; }
  .voice-heading span,p,a { font-size:11px; opacity:.72; }
  .voice-options,.combat-actions { display:grid; grid-template-columns:repeat(2,minmax(0,1fr)); gap:6px; }
  .btn { font-size:12px; padding-inline:8px; }
  .combat-actions .btn { text-align:left; }
  .combat-actions span { font-size:10px; opacity:.65; }
  .combat-volume { display:grid; gap:5px; font-size:12px; }
  .combat-volume span { display:flex; justify-content:space-between; }
  input { width:100%; accent-color:var(--color-gold); }
  p { margin:0; line-height:1.5; }
  a { color:var(--color-gold); text-decoration:underline; }
</style>
