<script lang="ts">
  import { onMount } from 'svelte';
  import { loadCatalog, trayTextures } from '../assets/catalog';
  import { DicePlayer } from '../dice/dicePlayer';
  import { damageApplication, damageThresholds, mayRollDamage } from '../dice/damageFlow';
  import { dieLook } from '../dice/palette';
  import { rollDuration } from '../dice/playback';
  import { impacts, playBounces, playRollSettle } from '../dice/sound';
  import { sound } from '../dice/sound.svelte';
  import { systems } from '../systems/store.svelte';
  import { prefs } from '../prefs.svelte';
  import { send } from '../net/socket';
  import { table } from '../table.svelte';

  let canvas: HTMLCanvasElement;
  let player: DicePlayer | undefined;
  let ready = $state(false);
  let pendingRollId = $state<string | null>(null);
  let damageError = $state<string | null>(null);
  let previousRollId: string | undefined;

  const showing = $derived(table.activeRoll);
  const landed = $derived(showing !== null && table.landedRollId === showing.id);
  const recorded = $derived(table.rolls.find((roll) => roll.id === showing?.id)?.contest ?? null);
  const blow = $derived(recorded?.blow ?? showing?.contest?.blow ?? null);
  const canDamage = $derived(mayRollDamage(showing, table.campaign.systemId, table.characters, table.role, table.playerId, table.viewAs, recorded));
  const thresholds = $derived(damageThresholds(blow));

  onMount(() => {
    player = new DicePlayer(canvas);
    const observer = new ResizeObserver(() => player?.resize());
    observer.observe(canvas);
    ready = true;
    void loadCatalog().then((packs) => {
      const tray = trayTextures(packs);
      player?.setTray(tray.felt, tray.wood);
    });
    return () => {
      ready = false;
      observer.disconnect();
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
      () => {
        if (table.activeRoll?.id === roll.id && !canDamage && pendingRollId !== roll.id) table.dismissRoll();
      },
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

  $effect(() => {
    if (showing?.id === previousRollId) return;
    previousRollId = showing?.id;
    pendingRollId = null;
    damageError = null;
  });

  $effect(() => {
    if (!pendingRollId) return;
    if (table.error || !table.connected) {
      damageError = table.error || 'Connection lost. Reconnect to try again.';
      pendingRollId = null;
    }
  });

  $effect(() => {
    const id = pendingRollId;
    if (!id) return;
    const timeout = window.setTimeout(() => {
      if (pendingRollId !== id) return;
      pendingRollId = null;
      damageError = recorded?.damageRolled
        ? 'Damage was accepted. Check Rolls if you reconnected.'
        : 'No response yet. You can safely try again.';
    }, 15_000);
    return () => window.clearTimeout(timeout);
  });

  function rollDamage() {
    if (!showing?.contest || !landed || !canDamage || pendingRollId || !table.connected) return;
    pendingRollId = showing.id;
    damageError = null;
    table.error = null;
    send({ type: 'strike', attackRollId: showing.id, attackerId: showing.contest.attackerId, targetId: showing.contest.targetId, plus: 0, minus: 0 });
  }

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
    {#if showing}
      <div class="dice-tray-heading">
        <svg aria-hidden="true" width="22" height="24" viewBox="0 0 24 26" fill="none" stroke="currentColor" stroke-width="1.2">
          <path d="M12 1 22 7v12l-10 6L2 19V7L12 1Z M12 1 7 9h10l-5-8ZM2 7l5 2-5 10m20-12-5 2 5 10M7 9l5 16 5-16M2 19l10-5 10 5M7 9l5 5 5-5" />
        </svg>
        <div class="min-w-0 flex-1">
          <p class="dice-tray-title">Dice tray</p>
          <p class="dice-tray-caption">{landed ? 'Roll resolved' : 'The dice are rolling'}</p>
        </div>
        <button class="dice-tray-close" type="button" aria-label="Close dice tray" title="Close dice tray" onclick={dismiss}>
          <svg aria-hidden="true" width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5"><path d="m4 4 8 8M12 4l-8 8" /></svg>
        </button>
      </div>
    {/if}
    <canvas bind:this={canvas} class="block min-h-0 w-full flex-1" onclick={dismiss}></canvas>
    {#if showing}
      <button class="dice-result flex w-full flex-wrap items-center gap-2 px-4 py-2 text-left" type="button" onclick={dismiss}>
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
            {:else if blow}
              <span class="text-base">
                {table.campaign.systemId === 'daggerheart' ? blow.step : blow.marks > 0 ? `${blow.step} · ${blow.marks} HP` : 'No mark'}
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
      {#if landed && table.campaign.systemId === 'daggerheart' && blow}
        <div class="dice-resolution shrink-0 border-t border-[var(--color-gold)]/20 px-4 py-2 text-sm" role="status">
          <p>{showing.total} damage{#if thresholds} · {thresholds}{/if}</p>
          <p class="mt-1 text-[var(--color-gold-bright)]">{damageApplication(blow)}</p>
        </div>
      {:else if landed && (canDamage || pendingRollId === showing.id || damageError)}
        <div class="dice-actions flex shrink-0 flex-wrap items-center gap-2 border-t border-[var(--color-gold)]/20 px-4 py-3">
          {#if canDamage || pendingRollId === showing.id}
            <button class="btn disabled:opacity-50" type="button" onclick={rollDamage} disabled={pendingRollId === showing.id || !table.connected}>
              {pendingRollId === showing.id ? 'Rolling damage…' : 'Roll damage'}
            </button>
          {/if}
          <span class="text-sm text-[var(--color-parchment)]/70">{!table.connected ? 'Reconnecting…' : `Hit ${showing.contest?.targetName ?? ''}`}</span>
          {#if damageError}<p class="w-full text-sm text-[#e7a090]" role="alert">{damageError}</p>{/if}
        </div>
      {/if}
    {/if}
  </div>
</div>
