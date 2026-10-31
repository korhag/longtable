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
  const size = 292;
  const left = $derived(Math.max(6, Math.min(viewportWidth - size - 6, (table.menu?.x ?? 0) - size / 2)));
  const top = $derived(Math.max(6, Math.min(viewportHeight - size - 40, (table.menu?.y ?? 0) - size / 2)));
  function close() { table.menu = null; }
  $effect(() => { table.menu; page = 0; preview = false; void tick().then(() => menuEl?.querySelector<HTMLButtonElement>('[role=menuitem]')?.focus()); });
  onMount(() => {
    const dismiss = (event: PointerEvent) => { if (event.button === 2) return; if (event.target instanceof Node && !menuEl?.contains(event.target)) close(); };
    window.addEventListener('pointerdown', dismiss);
    return () => window.removeEventListener('pointerdown', dismiss);
  });
  function key(event: KeyboardEvent) {
    if (event.key === 'Escape') { event.preventDefault(); close(); return; }
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

<svelte:window bind:innerWidth={viewportWidth} bind:innerHeight={viewportHeight} onkeydown={event => { if (table.menu && event.key === 'Escape') { event.preventDefault(); close(); } }} />
{#if table.menu && actions.length}
  <div bind:this={menuEl} class="context-dial" style={`left:${left}px;top:${top}px`} role="menu" aria-label={`Actions for ${clicked?.name ?? 'this location'}`} tabindex="-1" onkeydown={key} oncontextmenu={event => event.preventDefault()}>
    <div class="dial-ring" aria-hidden="true"></div>
    <button class="dial-center" type="button" onclick={close} aria-label="Close actions"><span>{clicked?.name ?? 'Move to here'}</span><small>{preview ? 'Local animation preview' : selected && selected.id !== clicked?.id ? selected.name : 'Character actions'}</small><b aria-hidden="true">×</b></button>
    {#each current as action, index (action.id)}
      {@const angle = -Math.PI / 2 + index * Math.PI / 3}
      <button class="dial-action" type="button" role="menuitem" title={action.detail ?? action.label} style={`left:${146 + Math.cos(angle) * 100}px;top:${146 + Math.sin(angle) * 100}px`} onclick={() => press(action.id)}><span aria-hidden="true">{action.icon}</span><strong>{action.label}</strong></button>
    {/each}
    {#if pages > 1}<button class="dial-page" type="button" onclick={() => page = (page + 1) % pages}>More actions · {page + 1}/{pages} →</button>{/if}
  </div>
{/if}
<style>
  .context-dial { position:fixed; z-index:var(--z-menu); width:292px; height:292px; pointer-events:auto; }
  .dial-ring { position:absolute; inset:7px; border-radius:50%; border:1px solid #b18c4770; background:radial-gradient(circle,transparent 28%,#221b13ee 29%,#17130ff5 65%,#a6803e55 66%,transparent 67%); filter:drop-shadow(0 8px 20px #0009); }
  .dial-center { position:absolute; left:146px; top:146px; transform:translate(-50%,-50%); width:112px; height:112px; border-radius:50%; border:1px solid #b18c4760; background:#17130ff2; color:var(--color-gold); display:flex; flex-direction:column; justify-content:center; align-items:center; padding:10px; gap:3px; cursor:pointer; }
  .dial-center span { font-size:11px; line-height:1.3; max-width:96px; }
  .dial-center small { opacity:.65; font-size:9px; line-height:1.2; }
  .dial-center b { font-size:17px; font-weight:400; }
  .dial-action { position:absolute; transform:translate(-50%,-50%); width:84px; min-height:54px; display:flex; flex-direction:column; justify-content:center; gap:2px; padding:6px 4px; border:1px solid #b18c4777; background:linear-gradient(#312718,#1c1711); border-radius:16px; color:#ead8b5; cursor:pointer; }
  .dial-action span { color:#d4a656; font-size:18px; line-height:20px; }
  .dial-action strong { font-size:11px; font-weight:500; line-height:1.15; }
  button:hover,button:focus-visible { outline:1px solid #f2d48a; background:#4b3920; color:#fff0d2; }
  .dial-page { position:absolute; top:292px; left:50%; transform:translateX(-50%); white-space:nowrap; padding:6px 12px; border:1px solid #b18c4770; border-radius:20px; color:#d4a656; background:#1c1711f5; font-size:11px; cursor:pointer; }
</style>
