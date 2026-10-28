<script lang="ts">
  // ------------------------------------------------------------
  // Component: PlayChrome
  // Purpose: Fear, the party, adversary portraits, the target
  //          card, and the right-click menu. Only mounted while
  //          Daggerheart is in Play.
  // ------------------------------------------------------------

  import { onMount } from 'svelte';

  import type { Figure } from '../../lib/net/bindings/Figure.ts';
  import type { TokenView } from '../../lib/net/bindings/TokenView.ts';
  import { send } from '../../lib/net/socket';
  import { systems } from '../../lib/systems/store.svelte';
  import { table } from '../../lib/table.svelte';
  import { attackFormula } from './attack';

  let { aimPoint, onCenter }: {
    aimPoint: { x: number; y: number } | null;
    onCenter: (x: number, y: number) => void;
  } = $props();

  const HEALTH = ['Uninjured', 'Slightly Wounded', 'Wounded', 'Badly Wounded', 'Near Death'];
  const STRESS = ['Clear', 'Uneasy', 'Strained', 'Stressed', 'Spent'];
  const HEALTH_COLORS = ['#6e9a72', '#c6b15a', '#d4a656', '#d27b66', '#c45c4a'];
  const STRESS_COLORS = ['#8a8478', '#7eb6e0', '#d4a656', '#d27b66', '#9d7ad6'];

  let menuEl = $state<HTMLDivElement | null>(null);

  const gm = $derived(table.role === 'gm' && !table.viewAs);
  const party = $derived(table.figures.filter((figure) => figure.kind === 'pc'));
  const foes = $derived(table.figures.filter((figure) => figure.kind !== 'pc'));
  const targetFace = $derived(table.figures.find((figure) => figure.tokenId === table.targetTokenId) ?? null);
  const menuToken = $derived(table.tokens.find((token) => token.id === table.menu?.tokenId) ?? null);
  const attacker = $derived.by(() => {
    const token = table.tokens.find((item) => item.id === table.selectedTokenId);
    return table.characters.find((item) => item.id === token?.characterId) ?? null;
  });
  const formula = $derived(attacker ? attackFormula(attacker) : null);
  const targetName = $derived(table.tokens.find((token) => token.id === table.targetTokenId)?.name ?? '');
  const showMenu = $derived(Boolean(table.menu && menuToken && (formula || gm)));

  onMount(() => {
    const close = (event: PointerEvent) => {
      if (event.button !== 0) return;
      const node = event.target;
      if (node instanceof Node && menuEl?.contains(node)) return;
      table.menu = null;
    };
    window.addEventListener('pointerdown', close);
    return () => window.removeEventListener('pointerdown', close);
  });

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

  function attack() {
    if (!attacker || !formula) return;
    send({
      type: 'roll',
      formula,
      visibility: 'public',
      label: targetName ? `Attack ${targetName}` : 'Attack',
    });
    table.menu = null;
  }

  function takeControl() {
    const token = menuToken;
    if (!token) return;
    table.selectedTokenId = token.id;
    table.targetTokenId = null;
    if (token.characterId) systems.selectedId = token.characterId;
    table.menu = null;
  }

  function toggleHidden() {
    const token = menuToken;
    if (!token) return;
    send({
      type: 'updateToken',
      tokenId: token.id,
      name: null,
      ownerId: null,
      clearOwner: false,
      hidden: !token.hidden,
      visionRadius: null,
      grantsVision: null,
      color: null,
      characterId: null,
      clearCharacter: false,
    });
    table.menu = null;
  }

  function cardStyle(point: { x: number; y: number } | null): string {
    if (!point || typeof window === 'undefined') return 'left:50%;top:4.75rem;transform:translateX(-50%)';
    const x = Math.min(window.innerWidth - 188, Math.max(8, point.x + 18));
    const y = Math.min(window.innerHeight - 110, Math.max(64, point.y - 12));
    return `left:${x}px;top:${y}px`;
  }

  function menuStyle(): string {
    if (!table.menu || typeof window === 'undefined') return 'left:8px;top:8px';
    const x = Math.min(window.innerWidth - 180, Math.max(8, table.menu.x));
    const y = Math.min(window.innerHeight - 160, Math.max(8, table.menu.y));
    return `left:${x}px;top:${y}px`;
  }
</script>

{#snippet faceButton(face: Figure, labeled: boolean)}
  {@const token = tokenOf(face.tokenId)}
  <button
    type="button"
    class="pointer-events-auto flex items-center gap-2 rounded-lg px-1 py-1 text-left hover:bg-white/10"
    title={face.name}
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
      <span class="min-w-0">
        <span class="block max-w-36 truncate text-xs">{face.name}</span>
        <span class="block text-xs text-[var(--color-gold-bright)]">Hope {face.hope ?? 0}</span>
        <span class="block max-w-36 truncate text-xs">{face.health}</span>
      </span>
    {/if}
  </button>
{/snippet}

<div class="pointer-events-none absolute inset-0">
  <div class="flex items-start gap-2 p-2">
    <div class="pointer-events-auto flex max-w-[24rem] flex-wrap gap-1" aria-label="Fear">
      {#each Array.from({ length: 12 }, (_, index) => index) as index (index)}
        <button
          type="button"
          class="h-8 w-8 rounded-full border border-[#9d7ad6] disabled:opacity-100"
          style={`background:${index < table.fear ? '#9d7ad6' : 'transparent'}`}
          disabled={!gm}
          aria-label={`Fear ${index + 1}${index < table.fear ? ', filled' : ''}`}
          onclick={() => setFear(index)}
        ></button>
      {/each}
    </div>
    {#if foes.length > 0}
      <div class="flex max-w-full gap-1 overflow-x-auto">
        {#each foes as face (face.tokenId)}
          {@render faceButton(face, false)}
        {/each}
      </div>
    {/if}
  </div>
  {#if party.length > 0}
    <div class="ml-2 flex w-max max-w-56 flex-col gap-1">
      {#each party as face (face.tokenId)}
        {@render faceButton(face, true)}
      {/each}
    </div>
  {/if}
</div>

{#if targetFace}
  <div class="glass pointer-events-none fixed z-[var(--z-float)] w-44 rounded-lg px-3 py-2 text-xs" style={cardStyle(aimPoint)}>
    <p class="truncate text-sm">{targetFace.name}</p>
    <p class="mt-1">{targetFace.health}</p>
    <p>{targetFace.stress}</p>
  </div>
{/if}

{#if showMenu && menuToken}
  <div
    bind:this={menuEl}
    class="glass fixed z-[var(--z-menu)] flex w-max min-w-40 max-w-56 flex-col gap-1 rounded-lg p-1.5"
    style={menuStyle()}
    role="menu"
    tabindex="-1"
    oncontextmenu={(event) => event.preventDefault()}
  >
    {#if formula}
      <button class="btn quiet" type="button" role="menuitem" onclick={attack}>
        {targetName ? `Attack ${targetName}` : 'Attack'}
      </button>
    {/if}
    {#if gm}
      {#if menuToken.id !== table.selectedTokenId}
        <button class="btn quiet" type="button" role="menuitem" onclick={takeControl}>Take control</button>
      {/if}
      <button class="btn quiet" type="button" role="menuitem" onclick={toggleHidden}>{menuToken.hidden ? 'Show' : 'Hide'}</button>
    {/if}
  </div>
{/if}
