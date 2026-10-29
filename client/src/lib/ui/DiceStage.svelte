<script lang="ts">
  import { onMount } from 'svelte';
  import { loadCatalog, trayTextures } from '../assets/catalog';
  import { DicePlayer } from '../dice/dicePlayer';
  import { dieLook } from '../dice/palette';
  import { rollDuration } from '../dice/playback';
  import { impacts, playBounces, playRollSettle } from '../dice/sound';
  import { sound } from '../dice/sound.svelte';
  import { systems } from '../systems/store.svelte';
  import { prefs } from '../prefs.svelte';
  import { table } from '../table.svelte';

  let canvas: HTMLCanvasElement;
  let player: DicePlayer | undefined;
  let ready = $state(false);

  const showing = $derived(table.activeRoll);
  const landed = $derived(showing !== null && table.landedRollId === showing.id);

  onMount(() => {
    player = new DicePlayer(canvas);
    ready = true;
    void loadCatalog().then((packs) => {
      const tray = trayTextures(packs);
      player?.setTray(tray.felt, tray.wood);
    });
    return () => {
      ready = false;
      player?.dispose();
    };
  });

  $effect(() => {
    if (!ready || !player) return;
    const roll = table.activeRoll;
    if (!roll) {
      player.stop();
      return;
    }
    const meshes = table.diceMeshes;
    const started = player.play(
      roll,
      meshes,
      () => table.clock.serverNow(),
      () => {
        table.finishRoll(roll.id);
        playRollSettle();
      },
      () => table.dismissRoll(),
      systems.active?.dieTags ?? [],
    );
    if (!started) return;
    const frames = Math.max(0, ...roll.dice.map((die) => die.keyframes.length));
    const ends = roll.playAt + rollDuration(frames, roll.keyframeDt) * 1000;
    if (!sound.muted && table.clock.serverNow() < ends) {
      for (const die of roll.dice) {
        playBounces(die.sides, impacts(die.keyframes, roll.keyframeDt), roll.playAt, () => table.clock.serverNow(), die.keyframes[0]?.p[0] ?? 0);
      }
    }
  });

  function dismiss() {
    if (showing) table.dismissRoll();
  }

  function chipText(sign: number, value: number) {
    return `${sign < 0 ? '−' : ''}${value}`;
  }

  function chipName(tag: string | null, sides: number) {
    const named = tag ? systems.active?.dieTags.find((item) => item.letter === tag) : undefined;
    return named?.name ?? `d${sides}`;
  }
</script>

<div class="dice-overlay" class:away={!showing} class:quarter={prefs.diceTray !== 'cover'} class:corner-top-left={prefs.diceCorner === 'top-left'} class:corner-top-right={prefs.diceCorner === 'top-right'} class:corner-bottom-left={prefs.diceCorner === 'bottom-left'} class:corner-bottom-right={prefs.diceCorner === 'bottom-right'}>
  <button class="dice-dim" type="button" aria-label="Dismiss the roll" onclick={dismiss}></button>
  <div class="dice-tray">
    <canvas bind:this={canvas} class="block min-h-0 w-full flex-1" onclick={dismiss}></canvas>
    {#if showing}
      <button class="flex w-full flex-wrap items-center gap-2 px-4 py-2 text-left" type="button" onclick={dismiss}>
        {#if showing.contest}
          <p class="text-base">{showing.contest.attackerName} {showing.contest.blow ? 'damages' : 'attacks'} {showing.contest.targetName}</p>
        {:else}
          <p class="text-sm">{showing.rollerName} · {showing.label || showing.formula}</p>
        {/if}
        {#if landed}
          <span class="ml-auto flex flex-wrap items-center justify-end gap-1.5">
            {#each showing.dice as die, index (`${die.sides}-${die.tag}-${index}`)}
              {@const look = dieLook(die.sides, die.tag, systems.active?.dieTags ?? [])}
              <span
                class="inline-flex items-baseline gap-1 rounded-full px-2 py-0.5"
                style={`background:${look.body};color:${look.ink}`}
              >
                <span class="text-sm tracking-wide">{chipName(die.tag, die.sides)}</span>
                <span class="display text-lg leading-none">{chipText(die.sign, die.value)}</span>
              </span>
            {/each}
            {#if showing.modifier !== 0}
              <span class="display text-lg text-[var(--color-parchment)]/80">
                {showing.modifier > 0 ? `+${showing.modifier}` : showing.modifier}
              </span>
            {/if}
            <span class="display text-3xl text-[var(--color-gold-bright)]">= {showing.total}</span>
            {#if showing.contest?.verdict}
              <span class="display text-2xl {showing.contest.verdict === 'hit' ? 'text-[var(--color-gold-bright)]' : 'text-[#e7a090]'}">
                {showing.contest.verdict === 'hit' ? 'Hit' : 'Miss'}
              </span>
              <span class="text-sm">{showing.total} vs {showing.contest.defenseName} {showing.contest.defense}</span>
            {:else if showing.contest?.blow}
              <span class="text-base">
                {showing.contest.blow.marks > 0 ? `${showing.contest.blow.step} · ${showing.contest.blow.marks} HP` : 'No mark'}
              </span>
            {/if}
            {#if showing.outcome}
              <span class="text-sm text-[var(--color-gold-bright)]">{showing.outcome.label}</span>
            {/if}
          </span>
        {:else}
          <p class="ml-auto text-sm tracking-[0.22em] text-[var(--color-gold)] uppercase">Rolling</p>
        {/if}
      </button>
    {/if}
  </div>
</div>
