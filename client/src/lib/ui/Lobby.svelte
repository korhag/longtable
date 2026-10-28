<script lang="ts">
  import { onMount } from 'svelte';
  import { loadHost, type HostInfo } from '../net/host';
  import { joinTable, shouldSkipAutoJoin } from '../net/socket';
  import { loadSeat, saveSeat, table, type SeatRequest } from '../table.svelte';
  import { VERSION, VERSION_NOTE } from '../version';

  const saved = loadSeat();
  let name = $state(saved?.name ?? '');
  let passphrase = $state(saved?.passphrase ?? '');
  let asGm = $state(saved?.asGm ?? false);
  let gmKey = $state(saved?.gmKey ?? '');
  let hostInfo = $state<HostInfo | null>(null);

  onMount(async () => {
    const host = await loadHost();
    hostInfo = host;
    if (host) {
      passphrase = host.passphrase;
      gmKey = host.gmKey;
      if (!saved) asGm = true;
    }
    if (shouldSkipAutoJoin() || !saved?.name) return;
    enter({
      name: saved.name,
      passphrase: host?.passphrase || saved.passphrase,
      asGm: saved.asGm,
      gmKey: saved.asGm ? host?.gmKey || saved.gmKey : saved.gmKey,
    });
  });

  function enter(seat: SeatRequest) {
    if (!seat.name || !seat.passphrase || (seat.asGm && !seat.gmKey)) return;
    saveSeat(seat);
    joinTable(seat);
  }

  function sitAsGm() {
    if (!hostInfo) return;
    enter({
      name: name.trim() || 'GM',
      passphrase: hostInfo.passphrase,
      asGm: true,
      gmKey: hostInfo.gmKey,
    });
  }

  function submit() {
    enter({ name: name.trim(), passphrase: passphrase.trim(), asGm, gmKey: gmKey.trim() });
  }
</script>

<div class="lobby fixed inset-0 grid place-items-center bg-[#100e0c]/80 p-6">
  <form class="glass w-full max-w-md rounded-3xl p-8" onsubmit={(event) => { event.preventDefault(); submit(); }}>
    <p class="text-xs tracking-[0.28em] text-[var(--color-gold)] uppercase">Take a seat</p>
    <h1 class="display mt-2 text-4xl text-[var(--color-gold-bright)]">Game Table</h1>
    <p class="mt-2 text-xs tracking-wide text-[var(--color-gold)]/80">v{VERSION} · {VERSION_NOTE}</p>
    <p class="mt-3 text-sm leading-relaxed text-[var(--color-parchment)]/75">The table is ready to play.</p>

    {#if hostInfo}
      <div class="mt-6 rounded-2xl border border-[rgba(212,166,86,0.35)] bg-black/25 p-4">
        <p class="text-sm">This computer is hosting the game.</p>
        <p class="display mt-2 text-center text-3xl tracking-[0.35em] text-[var(--color-gold-bright)]">{hostInfo.gmKey}</p>
        <button class="btn display mt-4 w-full" type="button" onclick={sitAsGm}>Sit as GM</button>
      </div>
    {/if}

    <label class="mt-6 block text-xs tracking-wide text-[var(--color-gold)]/80 uppercase">Name
      <input class="field mt-1" bind:value={name} maxlength="24" autocomplete="nickname" />
    </label>
    <label class="mt-3 block text-xs tracking-wide text-[var(--color-gold)]/80 uppercase">Table passphrase
      <input class="field mt-1" type="password" bind:value={passphrase} autocomplete="current-password" />
    </label>

    <label class="mt-4 flex items-center gap-2 text-sm">
      <input type="checkbox" bind:checked={asGm} />
      I am the GM
    </label>
    {#if asGm}
      <label class="mt-3 block text-xs tracking-wide text-[var(--color-gold)]/80 uppercase">GM key
        <input class="field mt-1" bind:value={gmKey} autocomplete="off" spellcheck="false" />
      </label>
    {/if}

    {#if table.error}
      <p class="mt-4 text-sm text-[#e7a090]">{table.error}</p>
    {/if}

    <button class="btn quiet display mt-6 w-full" type="submit">Enter the table</button>
  </form>
</div>
