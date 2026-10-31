<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { rollSetup } from '../dice/rollSetup.svelte';
  import { layout } from '../layout/layout.svelte';
  import { send } from '../net/socket';
  import { systems } from '../systems/store.svelte';
  import { table } from '../table.svelte';
  import { contextActions } from './contextActions';
  import type { CastAnimationKind } from '../render/castAnimation';
  let { onCenter, onMove, onTeleport, onAnimate }: {
    onCenter: (x: number, y: number) => void;
    onMove: (id: string, x: number, y: number) => void;
    onTeleport: (id: string, x: number, y: number) => void;
    onAnimate: (id: string, kind: CastAnimationKind) => void;
  } = $props();
  let menuEl = $state<HTMLDivElement>();
  let previousFocus: HTMLElement | null = null;
  let page = $state(0);
  let preview = $state(false);
  const previews: { id: CastAnimationKind; label: string; icon: string; detail: string }[] = [
    { id: 'idle', label: 'Idle', icon: '◌', detail: 'Local preview · breathing' },
    { id: 'melee', label: 'Melee attack', icon: '⚔', detail: 'Local preview · wind-up, strike, recovery' },
    { id: 'ranged', label: 'Ranged attack', icon: '➶', detail: 'Local preview · aim, shot, recoil' },
    { id: 'move', label: 'Move', icon: '↗', detail: 'Local preview · stride in place' },
    { id: 'hurt', label: 'Take damage', icon: '✴', detail: 'Local preview · flinch' },
    { id: 'spell', label: 'Cast a spell', icon: '✦', detail: 'Local preview · gesture and magical glow' },
  ];
  let viewportWidth = $state(1400), viewportHeight = $state(900);
  const gm = $derived(table.role === 'gm' && !table.viewAs);
  const clicked = $derived(table.tokens.find(token => token.id === table.menu?.tokenId) ?? null);
  const selected = $derived(table.tokens.find(token => token.id === table.selectedTokenId) ?? null);
  const actor = $derived(table.characters.find(character => character.id === selected?.characterId) ?? null);
  const clickedCharacter = $derived(table.characters.find(character => character.id === clicked?.characterId) ?? null);
  const target = $derived(clicked && clicked.id !== selected?.id ? clicked : table.tokens.find(token => token.id === table.targetTokenId) ?? null);
  const actions = $derived(contextActions({ clicked, selected, actor, target, system: systems.active, gm, seat: table.playerId, readOnly: !!table.viewAs, hasPoint: !!table.menu?.point, hasClickedSheet: !!clickedCharacter }));
  const pages = $derived(preview ? 1 : Math.max(1, Math.ceil(actions.length / 6)));
  const current = $derived(preview ? previews : actions.slice((page % pages) * 6, (page % pages + 1) * 6));
  const size = $derived(Math.min(308, Math.max(252, viewportWidth - 12)));
  const left = $derived(Math.max(6, Math.min(viewportWidth - size - 6, (table.menu?.x ?? 0) - size / 2)));
  const top = $derived(Math.max(6, Math.min(viewportHeight - size - 40, (table.menu?.y ?? 0) - size / 2)));
  const compassTicks = Array.from({ length: 24 }, (_, index) => index * Math.PI / 12);
  function close(restoreFocus = false) {
    table.menu = null;
    if (restoreFocus && previousFocus?.isConnected) previousFocus.focus({ preventScroll: true });
  }
  function back() { preview = false; page = 0; void tick().then(() => menuEl?.querySelector<HTMLButtonElement>('[role=menuitem]')?.focus()); }
  $effect(() => {
    const menu = table.menu;
    page = 0; preview = false;
    if (!menu) return;
    if (document.activeElement instanceof HTMLElement && !menuEl?.contains(document.activeElement)) previousFocus = document.activeElement;
    void tick().then(() => menuEl?.querySelector<HTMLButtonElement>('[role=menuitem]')?.focus());
  });
  onMount(() => {
    const dismiss = (event: PointerEvent) => { if (event.button === 2) return; if (event.target instanceof Node && !menuEl?.contains(event.target)) close(); };
    const escape = (event: KeyboardEvent) => {
      if (event.defaultPrevented || !table.menu || event.key !== 'Escape') return;
      event.preventDefault(); event.stopPropagation(); close(true);
    };
    window.addEventListener('pointerdown', dismiss);
    document.addEventListener('keydown', escape, true);
    return () => {
      window.removeEventListener('pointerdown', dismiss);
      document.removeEventListener('keydown', escape, true);
    };
  });
  function key(event: KeyboardEvent) {
    if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); close(true); return; }
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) return;
    const buttons = [...(menuEl?.querySelectorAll<HTMLButtonElement>('button') ?? [])];
    const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? buttons.length - 1 : (current + (event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : 1) + buttons.length) % buttons.length;
    event.preventDefault(); buttons[next]?.focus();
  }
  function press(id: string) {
    if (id === 'animations') { preview = true; page = 0; void tick().then(() => menuEl?.querySelector<HTMLButtonElement>('[role=menuitem]')?.focus()); return; }
    if (preview && clicked) { onAnimate(clicked.id, id as CastAnimationKind); close(); return; }
    const point = table.menu?.point;
    if (id === 'move' && selected && point) onMove(selected.id, point.x, point.y);
    if (id === 'teleport' && selected && point && gm) onTeleport(selected.id, point.x, point.y);
    if (id === 'center-point' && point) onCenter(point.x, point.y);
    if (id === 'center' && clicked) onCenter(clicked.x, clicked.y);
    if (id === 'select' && clicked) {
      table.selectedTokenId = clicked.id; table.targetTokenId = null;
      if (clicked.characterId) systems.selectedId = clicked.characterId;
    }
    if (id === 'target' && clicked) table.targetTokenId = clicked.id;
    if (id === 'sheet' && clickedCharacter) {
      systems.selectedId = clickedCharacter.id; systems.sheetOpen = true;
      const place = layout.place('sheet');
      layout.patch('sheet', { floating: true, collapsed: false, w: Math.max(place.w, 600), h: Math.max(place.h, 560) });
    }
    if (id === 'hide' && clicked && gm) send({ type: 'updateToken', tokenId: clicked.id, name: null, ownerId: null, clearOwner: false, hidden: !clicked.hidden, visionRadius: null, grantsVision: null, color: null, characterId: null, clearCharacter: false, setCastAsset: false, castAsset: null, facing: null });
    if (id === 'attack' && actor && target?.characterId) send({ type: 'attack', attackerId: actor.id, targetId: target.characterId, plus: rollSetup.plus, minus: rollSetup.minus });
    if (id.startsWith('rule:') && actor) send({ type: 'rollAction', characterId: actor.id, actionId: id.slice(5), visibility: rollSetup.visibility, plus: rollSetup.plus, minus: rollSetup.minus });
    close();
  }
</script>

<svelte:window bind:innerWidth={viewportWidth} bind:innerHeight={viewportHeight} />
{#if table.menu && actions.length}
  <div bind:this={menuEl} class="context-dial" style={`left:${left}px;top:${top}px;width:${size}px;height:${size}px`} role="menu" aria-label={`Actions for ${clicked?.name ?? 'this location'}`} tabindex="-1" onkeydown={key} oncontextmenu={event => event.preventDefault()}>
    <div class="dial-ring" aria-hidden="true"></div>
    <svg class="dial-compass" viewBox="0 0 308 308" aria-hidden="true">
      <circle cx="154" cy="154" r="149" />
      <circle cx="154" cy="154" r="139" />
      {#each compassTicks as angle, index}
        <line x1={154 + Math.cos(angle) * (index % 6 === 0 ? 133 : 143)} y1={154 + Math.sin(angle) * (index % 6 === 0 ? 133 : 143)} x2={154 + Math.cos(angle) * 149} y2={154 + Math.sin(angle) * 149} />
      {/each}
    </svg>
    <button class="dial-center" type="button" onclick={() => close(true)} aria-label="Close actions" title="Close actions (Esc)">
      <small class="dial-kicker">{preview ? 'Animation' : 'Take action'}</small>
      <span>{clicked?.name ?? 'This location'}</span>
      {#if !preview && selected && selected.id !== clicked?.id}<small class="dial-actor">As {selected.name}</small>{/if}
      <b aria-hidden="true">×</b>
    </button>
    {#each current as action, index (action.id)}
      {@const angle = -Math.PI / 2 + index * Math.PI / 3}
      <button class="dial-action" type="button" role="menuitem" title={action.detail ?? action.label} style={`left:${50 + Math.cos(angle) * 36.4}%;top:${50 + Math.sin(angle) * 36.4}%`} onclick={() => press(action.id)}><span aria-hidden="true">{action.icon}</span><strong>{action.label}</strong></button>
    {/each}
    {#if preview}<button class="dial-page" type="button" onclick={back}>← Character actions</button>
    {:else if pages > 1}<button class="dial-page" type="button" onclick={() => page = (page + 1) % pages}>More actions · {page + 1}/{pages} →</button>
    {:else}<span class="dial-hint">Choose an action · Esc to close</span>{/if}
  </div>
{/if}
<style>
  .context-dial { position:fixed; z-index:var(--z-menu); pointer-events:auto; }
  .dial-ring { position:absolute; inset:3px; border-radius:50%; background:radial-gradient(circle,#30251b 0 25%,#17130f 26% 49%,#211a13 50% 68%,#0e0c0a 69%); box-shadow:0 14px 30px #0009,inset 0 1px 0 #f4d59024; }
  .dial-compass { position:absolute; inset:0; width:100%; height:100%; fill:none; stroke:#ac87454d; stroke-width:1; pointer-events:none; }
  .dial-center { position:absolute; left:50%; top:50%; transform:translate(-50%,-50%); width:34%; height:34%; border-radius:50%; border:1px solid #b18c4777; background:radial-gradient(circle at 40% 20%,#33271c,#1c1611 76%); color:#ead7b3; display:flex; flex-direction:column; justify-content:center; align-items:center; padding:9px; gap:4px; box-shadow:inset 0 0 0 3px #14100c,inset 0 0 0 4px #a67b3830; }
  .dial-center span { font-family:var(--font-display); font-size:10px; line-height:1.35; overflow-wrap:anywhere; }
  .dial-kicker { font-size:8px; line-height:1; letter-spacing:.13em; text-transform:uppercase; color:#c79d60; }
  .dial-actor { max-width:100%; font-size:9px; line-height:1.2; color:#b7a78e; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
  .dial-center b { font-size:16px; font-weight:400; line-height:12px; color:#ab9064; }
  .dial-action { position:absolute; transform:translate(-50%,-50%); width:28%; min-height:60px; display:flex; flex-direction:column; align-items:center; justify-content:center; gap:4px; padding:7px 5px; border:1px solid #a88247aa; background:linear-gradient(160deg,#3c2d1e,#211910 75%); border-radius:7px; color:#ead8b5; box-shadow:0 3px 8px #0008,inset 0 1px 0 #edcf8c26; transition:border-color .12s,color .12s,background .12s; }
  .dial-action::after { content:''; position:absolute; inset:3px; border:1px solid #cda25818; border-radius:4px; pointer-events:none; }
  .dial-action span { color:#dab675; font-size:21px; line-height:21px; }
  .dial-action strong { font-size:11px; font-weight:500; line-height:1.18; }
  button:hover { border-color:#e2bd78; background:#44331f; color:#fff0d2; }
  button:focus-visible { outline:2px solid #f2d48a; outline-offset:3px; background:#44331f; color:#fff0d2; }
  .dial-page,.dial-hint { position:absolute; top:calc(100% + 5px); left:50%; transform:translateX(-50%); white-space:nowrap; padding:6px 12px; border:1px solid #b18c4760; border-radius:5px; color:#d3b784; background:#1c1711f5; font-size:11px; }
  .dial-hint { color:#b3a38b; border-color:#b18c4730; font-size:10px; pointer-events:none; }
</style>
