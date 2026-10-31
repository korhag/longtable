<script lang="ts">
  // ------------------------------------------------------------
  // Component: PlayChrome
  // Purpose: Fear, the party, adversary portraits, the target
  //          card. Context actions live in the shared dial. Mounted while
  //          Daggerheart is in Play.
  // ------------------------------------------------------------

  import type { Figure } from '../../lib/net/bindings/Figure.ts';
  import type { TokenView } from '../../lib/net/bindings/TokenView.ts';
  import { send } from '../../lib/net/socket';
  import { table } from '../../lib/table.svelte';
  import RuleBook from './RuleBook.svelte';

  let { aimPoint, onCenter }: {
    aimPoint: { x: number; y: number } | null;
    onCenter: (x: number, y: number) => void;
  } = $props();

  const HEALTH = ['Uninjured', 'Slightly Wounded', 'Wounded', 'Badly Wounded', 'Near Death'];
  const STRESS = ['Clear', 'Uneasy', 'Strained', 'Stressed', 'Spent'];
  const HEALTH_COLORS = ['#6e9a72', '#c6b15a', '#d4a656', '#d27b66', '#c45c4a'];
  const STRESS_COLORS = ['#8a8478', '#7eb6e0', '#d4a656', '#d27b66', '#9d7ad6'];

  let book = $state(false);

  const gm = $derived(table.role === 'gm' && !table.viewAs);
  const party = $derived(table.figures.filter((figure) => figure.kind === 'pc'));
  const foes = $derived(table.figures.filter((figure) => figure.kind !== 'pc'));
  const targetFace = $derived(table.figures.find((figure) => figure.tokenId === table.targetTokenId) ?? null);

  function initials(name: string): string {
    return name
      .trim()
      .split(/\s+/)
      .slice(0, 2)
      .map((part) => part[0] ?? '')
      .join('')
      .toUpperCase();
  }

  function dot(word: string, words: string[], colors: string[]): string {
    const index = words.indexOf(word);
    return colors[index < 0 ? 0 : index] ?? colors[0] ?? '#8a8478';
  }

  function tokenOf(tokenId: string): TokenView | null {
    return table.tokens.find((token) => token.id === tokenId) ?? null;
  }

  function hover(tokenId: string | null) {
    table.hoverTokenId = tokenId;
  }

  function aim(tokenId: string) {
    table.targetTokenId = tokenId;
    table.menu = null;
    const token = tokenOf(tokenId);
    if (token) onCenter(token.x, token.y);
  }

  function setFear(index: number) {
    const next = table.fear === index + 1 ? index : index + 1;
    const delta = next - table.fear;
    if (delta !== 0) send({ type: 'adjustFear', delta });
  }

  function cardStyle(point: { x: number; y: number } | null): string {
    if (!point || typeof window === 'undefined') return 'left:50%;top:4.75rem;transform:translateX(-50%)';
    const x = Math.min(window.innerWidth - 188, Math.max(8, point.x + 18));
    const y = Math.min(window.innerHeight - 110, Math.max(64, point.y - 12));
    return `left:${x}px;top:${y}px`;
  }

</script>

{#snippet faceButton(face: Figure, labeled: boolean)}
  {@const token = tokenOf(face.tokenId)}
  <button
    type="button"
    class="figure-button pointer-events-auto flex items-center gap-2 rounded-lg px-1 py-1 text-left hover:bg-white/10"
    title={`${face.name} · ${face.health} · ${face.stress}${labeled ? ` · Hope ${face.hope ?? 0}` : ''}`}
    aria-label={`${face.name} · ${face.health} · ${face.stress}${labeled ? ` · Hope ${face.hope ?? 0}` : ''}`}
    aria-pressed={table.targetTokenId === face.tokenId}
    onmouseenter={() => hover(face.tokenId)}
    onmouseleave={() => { if (table.hoverTokenId === face.tokenId) hover(null); }}
    onclick={() => aim(face.tokenId)}
  >
    <span class="portrait relative shrink-0" style={`--ring:${token?.color ?? '#d4a656'};opacity:${token?.hidden ? 0.45 : 1}`}>
      {initials(face.name)}
      <span class="absolute -bottom-0.5 left-0 h-2.5 w-2.5 rounded-full border border-black" style={`background:${dot(face.health, HEALTH, HEALTH_COLORS)}`}></span>
      <span class="absolute -bottom-0.5 right-0 h-2.5 w-2.5 rounded-full border border-black" style={`background:${dot(face.stress, STRESS, STRESS_COLORS)}`}></span>
    </span>
    {#if labeled}
      <span class="figure-label min-w-0">
        <span class="block max-w-36 truncate text-sm">{face.name}</span>
        <span class="block text-sm text-[var(--color-gold-bright)]">Hope {face.hope ?? 0}</span>
        <span class="block max-w-36 truncate text-sm">{face.health}</span>
      </span>
    {/if}
  </button>
{/snippet}

<div class="pointer-events-none absolute inset-0">
  <div class="play-status-row flex items-start gap-2 p-2">
    <div class="fear-track pointer-events-auto flex shrink-0 items-center gap-2 rounded-full border border-[#c4a2ee]/40 bg-[#120c18]/90 px-2.5 py-1 shadow-[0_10px_28px_rgba(0,0,0,0.4),inset_0_1px_0_rgba(255,255,255,0.08)]" aria-label="Fear">
      <span class="fear-label pl-1 text-sm tracking-[0.2em] text-[#e4d4f8] uppercase">Fear</span>
      <span class="fear-pips flex flex-nowrap items-center gap-[5px]">
        {#each Array.from({ length: 12 }, (_, index) => index) as index (index)}
          <button
            type="button"
            class="h-[1.15rem] w-[1.15rem] shrink-0 rounded-full border transition disabled:opacity-100 enabled:hover:brightness-125 {index < table.fear
              ? 'border-[#f4ecff] bg-[#b794f6] shadow-[0_0_8px_rgba(183,148,246,0.9),inset_0_1px_1px_rgba(255,255,255,0.55)]'
              : 'border-[#9d7ad6]/75 bg-[#1c1228] shadow-[inset_0_1px_2px_rgba(0,0,0,0.6)]'}"
            disabled={!gm}
            aria-label={`Fear ${index + 1}${index < table.fear ? ', filled' : ''}`}
            onclick={() => setFear(index)}
          ></button>
        {/each}
      </span>
      <span class="fear-value display min-w-[1.1rem] pr-0.5 text-center text-sm text-[#f2d48a]">{table.fear}</span>
    </div>
    <button class="rules-button btn quiet pointer-events-auto px-2 py-1 text-sm" type="button" onclick={() => (book = true)}>Rules</button>
    {#if foes.length > 0}
      <div class="adversary-roster flex max-w-full gap-1 overflow-x-auto">
        {#each foes as face (face.tokenId)}
          {@render faceButton(face, false)}
        {/each}
      </div>
    {/if}
  </div>
  {#if party.length > 0}
    <div class="party-roster ml-2 flex w-max max-w-56 flex-col gap-1">
      {#each party as face (face.tokenId)}
        {@render faceButton(face, true)}
      {/each}
    </div>
  {/if}
</div>

{#if book}
  <RuleBook close={() => (book = false)} />
{/if}

<style>
  .adversary-roster { min-width: 0; pointer-events: auto; scrollbar-width: thin; }
  .adversary-roster .figure-button { flex-shrink: 0; }
  @media (max-width: 760px) {
    .play-status-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 4px 6px; max-width: 100%; }
    .fear-track { min-width: 0; display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 5px; padding: 5px 7px; border-radius: 8px; }
    .fear-label { padding-left: 0; font-size: 10px; letter-spacing: .12em; }
    .fear-pips { display: grid; grid-template-columns: repeat(6, 16px); justify-content: center; gap: 3px; }
    .fear-pips button { width: 16px; height: 16px; }
    .fear-value { min-width: 14px; padding: 0; font-size: 12px; }
    .rules-button { padding: 6px; font-size: 11px; }
    .adversary-roster { grid-column: 1 / -1; width: 100%; overscroll-behavior-x: contain; }
    .figure-button { gap: 0; min-height: 44px; padding: 3px; }
    .figure-button .portrait { width: 36px; height: 36px; font-size: 11px; }
    .party-roster { gap: 2px; max-width: 44px; }
    .party-roster .figure-label { display: none; }
  }
</style>

{#if targetFace}
  <div class="glass pointer-events-none fixed z-[var(--z-float)] w-44 rounded-lg px-3 py-2 text-sm" style={cardStyle(aimPoint)}>
    <p class="truncate text-sm">{targetFace.name}</p>
    <p class="mt-1">{targetFace.health}</p>
    <p>{targetFace.stress}</p>
  </div>
{/if}
