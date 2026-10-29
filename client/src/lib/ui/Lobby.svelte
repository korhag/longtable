<script lang="ts">
  import { onMount } from 'svelte';
  import { account } from '../account.svelte';
  import { loadHost, type HostInfo } from '../net/host';
  import { joinTable, shouldSkipAutoJoin } from '../net/socket';
  import { saveSeat, table } from '../table.svelte';
  import { VERSION, VERSION_NOTE } from '../version';

  let hostInfo = $state<HostInfo | null>(null);
  let username = $state('');
  let password = $state('');
  let creating = $state(false);
  let tableName = $state('');
  let mode = $state<'in' | 'new'>('in');
  let busy = $state(false);

  onMount(async () => {
    hostInfo = await loadHost();
    if (!account.token || shouldSkipAutoJoin()) return;
    await account.refresh();
    const lobby = account.lobby;
    if (!lobby) return;
    const open = lobby.tables.find((item) => item.id === lobby.openId && (item.yours || item.standing === 'always'));
    if (open) sit(open.id);
  });

  function sit(tableId: string) {
    const name = account.lobby?.account.username ?? 'Guest';
    saveSeat({
      name,
      passphrase: 'account',
      asGm: false,
      gmKey: '',
      accountToken: account.token ?? '',
      tableId,
    });
    joinTable({
      name,
      passphrase: 'account',
      asGm: false,
      gmKey: '',
      accountToken: account.token ?? '',
      tableId,
    });
  }

  async function signIn(event: Event) {
    event.preventDefault();
    busy = true;
    account.error = null;
    try {
      if (mode === 'new') await account.register(username.trim(), password);
      else await account.login(username.trim(), password);
    } catch (err) {
      account.error = err instanceof Error ? err.message : 'that did not work';
    } finally {
      busy = false;
    }
  }

  async function asHost() {
    if (!hostInfo) return;
    busy = true;
    try {
      await account.login('host', hostInfo.passphrase);
      const open = account.lobby?.openId;
      if (open) sit(open);
    } catch (err) {
      account.error = err instanceof Error ? err.message : 'that did not work';
    } finally {
      busy = false;
    }
  }

  async function makeTable() {
    const name = tableName.trim();
    if (!name) return;
    tableName = '';
    creating = false;
    await account.createTable(name);
  }
</script>

<div class="lobby fixed inset-0 grid place-items-center overflow-auto bg-[#100e0c]/80 p-6">
  <div class="glass w-full max-w-md rounded-3xl p-8">
    <p class="text-sm tracking-[0.28em] text-[var(--color-gold)] uppercase">Take a seat</p>
    <h1 class="display mt-2 text-4xl text-[var(--color-gold-bright)]">Game Table</h1>
    <p class="mt-2 text-sm tracking-wide text-[var(--color-gold)]/80">v{VERSION} · {VERSION_NOTE}</p>

    {#if !account.lobby}
      {#if hostInfo}
        <div class="mt-6 rounded-2xl border border-[rgba(212,166,86,0.35)] bg-black/25 p-4">
          <p class="text-sm">This computer already has a table. Continue as its host.</p>
          <button class="btn display mt-4 w-full" type="button" disabled={busy} onclick={asHost}>Continue as host</button>
        </div>
      {/if}
      <form class="mt-6" onsubmit={signIn}>
        <div class="flex gap-2">
          <button class="btn quiet px-2 py-1 text-sm" type="button" class:active={mode === 'in'} onclick={() => (mode = 'in')}>Sign in</button>
          <button class="btn quiet px-2 py-1 text-sm" type="button" class:active={mode === 'new'} onclick={() => (mode = 'new')}>Create account</button>
        </div>
        <label class="mt-4 block text-sm tracking-wide text-[var(--color-gold)]/80 uppercase">Name
          <input class="field mt-1" bind:value={username} maxlength="24" autocomplete="username" />
        </label>
        <label class="mt-3 block text-sm tracking-wide text-[var(--color-gold)]/80 uppercase">Password
          <input class="field mt-1" type="password" bind:value={password} autocomplete={mode === 'new' ? 'new-password' : 'current-password'} />
        </label>
        <button class="btn quiet display mt-6 w-full" type="submit" disabled={busy}>{mode === 'new' ? 'Create account' : 'Sign in'}</button>
      </form>
    {:else}
      <p class="mt-4 text-sm">Signed in as {account.lobby.account.username}.</p>
      <div class="mt-4 flex flex-col gap-3">
        {#each account.lobby.tables as item (item.id)}
          <div class="rounded-2xl border border-[rgba(212,166,86,0.28)] p-3">
            <p class="display text-base">{item.name}</p>
            <p class="text-sm text-[var(--color-parchment)]/70">{item.yours ? 'Your table' : item.standing}</p>
            <div class="mt-2 flex flex-wrap gap-2">
              {#if item.yours && item.id !== account.lobby.openId}
                <button class="btn quiet px-2 py-1 text-sm" type="button" onclick={() => account.openTable(item.id)}>Open</button>
              {/if}
              {#if item.id === account.lobby.openId && (item.yours || item.standing === 'always' || item.standing === 'once')}
                <button class="btn px-2 py-1 text-sm" type="button" onclick={() => sit(item.id)}>Sit down</button>
              {/if}
            </div>
          </div>
        {/each}
        {#each account.lobby.friendTables as item (item.id)}
          <div class="rounded-2xl border border-[rgba(212,166,86,0.28)] p-3">
            <p class="display text-base">{item.ownerName}'s {item.name}</p>
            <button class="btn quiet mt-2 px-2 py-1 text-sm" type="button" onclick={() => account.requestSeat(item.id)}>Request a seat</button>
          </div>
        {/each}
        {#each account.lobby.requests as request (`${request.tableId}-${request.accountId}`)}
          <div class="rounded-2xl border border-[rgba(212,166,86,0.28)] p-3 text-sm">
            <p>{request.username} wants a seat at {request.tableName}.</p>
            <div class="mt-2 flex gap-2">
              <button class="btn quiet px-2 py-1" type="button" onclick={() => account.approve(request.tableId, request.accountId, false)}>This time</button>
              <button class="btn px-2 py-1" type="button" onclick={() => account.approve(request.tableId, request.accountId, true)}>Always</button>
            </div>
          </div>
        {/each}
        {#each account.lobby.incoming as person (person.id)}
          <div class="flex items-center justify-between gap-2 text-sm">
            <span>{person.username} wants to be friends.</span>
            <button class="btn quiet px-2 py-1" type="button" onclick={() => account.acceptFriend(person.id)}>Accept</button>
          </div>
        {/each}
      </div>
      {#if creating}
        <form class="mt-4 flex gap-2" onsubmit={(event) => { event.preventDefault(); void makeTable(); }}>
          <input class="field" placeholder="Table name" bind:value={tableName} maxlength="48" />
          <button class="btn px-3" type="submit">Create</button>
        </form>
      {:else}
        <button class="btn quiet mt-4 w-full" type="button" onclick={() => (creating = true)}>New table</button>
      {/if}
      <button class="btn quiet mt-3 w-full" type="button" onclick={() => account.clear()}>Sign out</button>
    {/if}

    {#if account.error || table.error}
      <p class="mt-4 text-sm text-[#e7a090]">{account.error || table.error}</p>
    {/if}
  </div>
</div>
