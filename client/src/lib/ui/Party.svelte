<script lang="ts">
  import { send } from '../net/socket';
  import { table } from '../table.svelte';

  const mine = $derived(table.players.find((player) => player.id === table.playerId));
</script>

<aside class="pointer-events-auto fixed top-4 left-4 z-10 flex max-w-[70vw] items-start gap-3">
  {#each table.players as player (player.id)}
    <div class="flex items-center gap-2">
      <div class="portrait relative" style={`--ring:${player.color}`}>
        {player.name.slice(0, 2).toUpperCase()}
        <span
          class={`absolute right-0 bottom-0 h-2.5 w-2.5 rounded-full border border-black ${player.connected ? 'bg-emerald-400' : 'bg-stone-500'}`}
        ></span>
      </div>
      <div>
        <p class="text-sm leading-none">{player.name}</p>
        <p class="mt-1 text-sm tracking-[0.16em] text-[var(--color-gold)]/80 uppercase">
          {player.role === 'gm' ? 'GM' : 'Player'}
          {#if player.id === table.playerId} · you{/if}
        </p>
      </div>
      {#if table.role === 'gm' && !table.viewAs && player.id !== table.playerId && !player.connected}
        <button
          class="btn quiet px-2 py-1 text-sm"
          type="button"
          onclick={() => send({ type: 'removePlayer', playerId: player.id })}
        >Dismiss</button>
      {/if}
    </div>
  {/each}
  {#if mine}
    <span class="sr-only">{mine.name}</span>
  {/if}
</aside>
