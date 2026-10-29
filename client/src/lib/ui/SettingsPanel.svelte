<script lang="ts">
  import { account } from '../account.svelte';
  import { prefs, THEMES } from '../prefs.svelte';

  const palette = ['#d4a656', '#7eb8a2', '#d27b66', '#8aa6d4', '#c98bb8', '#d8c7a1'];
  const corners = [
    { id: 'top-left', name: 'Top left' },
    { id: 'top-right', name: 'Top right' },
    { id: 'bottom-left', name: 'Bottom left' },
    { id: 'bottom-right', name: 'Bottom right' },
  ];

  let name = $state(account.lobby?.account.username ?? '');
  let friendName = $state('');
  let section = $state<'appearance' | 'play' | 'account'>('appearance');
  let note = $state<string | null>(null);

  $effect(() => {
    name = account.lobby?.account.username ?? name;
  });

  async function keep() {
    note = null;
    try {
      await account.update({ settings: prefs.current });
      note = 'Saved on your account.';
    } catch (err) {
      note = err instanceof Error ? err.message : 'Could not save.';
    }
  }

  async function rename() {
    note = null;
    try {
      await account.update({ username: name.trim() });
      note = 'Saved on your account.';
    } catch (err) {
      note = err instanceof Error ? err.message : 'Could not save.';
    }
  }

  async function paint(color: string) {
    try {
      await account.update({ color });
    } catch (err) {
      note = err instanceof Error ? err.message : 'Could not save.';
    }
  }

  function theme(id: string) {
    prefs.useTheme(id);
    void keep();
  }

  function tray(value: string) {
    prefs.diceTray = value;
    void keep();
  }

  function corner(value: string) {
    prefs.diceCorner = value;
    void keep();
  }

  function parchment(value: string) {
    prefs.parchment = value;
    prefs.adopt(prefs.current);
  }

  function gold(value: string) {
    prefs.gold = value;
    prefs.adopt(prefs.current);
  }
</script>

<div class="flex flex-col gap-3 text-sm">
  <div class="flex flex-wrap gap-2">
    <button class="btn quiet px-2 py-1" type="button" class:active={section === 'appearance'} onclick={() => (section = 'appearance')}>Appearance</button>
    <button class="btn quiet px-2 py-1" type="button" class:active={section === 'play'} onclick={() => (section = 'play')}>Play</button>
    <button class="btn quiet px-2 py-1" type="button" class:active={section === 'account'} onclick={() => (section = 'account')}>Account</button>
  </div>

  {#if section === 'appearance'}
    <p>The hearth look is the one this table started with.</p>
    <div class="flex flex-wrap gap-2">
      {#each Object.keys(THEMES) as id (id)}
        <button class="btn quiet px-2 py-1 capitalize" type="button" class:active={prefs.theme === id} onclick={() => theme(id)}>{id}</button>
      {/each}
    </div>
    <label class="flex items-center justify-between gap-3">Parchment
      <input type="color" value={prefs.parchment} oninput={(event) => parchment(event.currentTarget.value)} onchange={() => keep()} />
    </label>
    <label class="flex items-center justify-between gap-3">Gold
      <input type="color" value={prefs.gold} oninput={(event) => gold(event.currentTarget.value)} onchange={() => keep()} />
    </label>
  {:else if section === 'play'}
    <p>The dice can cover the map, or sit in one quarter of it.</p>
    <div class="flex flex-wrap gap-2">
      <button class="btn quiet px-2 py-1" type="button" class:active={prefs.diceTray === 'quarter'} onclick={() => tray('quarter')}>One quarter</button>
      <button class="btn quiet px-2 py-1" type="button" class:active={prefs.diceTray === 'cover'} onclick={() => tray('cover')}>Full map</button>
    </div>
    {#if prefs.diceTray === 'quarter'}
      <div class="flex flex-wrap gap-2">
        {#each corners as item (item.id)}
          <button class="btn quiet px-2 py-1" type="button" class:active={prefs.diceCorner === item.id} onclick={() => corner(item.id)}>{item.name}</button>
        {/each}
      </div>
    {/if}
  {:else}
    <label>Name
      <input class="field mt-1" bind:value={name} maxlength="24" />
    </label>
    <button class="btn quiet px-2 py-1" type="button" onclick={rename}>Save name</button>
    <p>Your color is the ring on your characters.</p>
    <div class="flex flex-wrap gap-2">
      {#each palette as color (color)}
        <button
          class="h-8 w-8 rounded-full border border-black/40"
          style={`background:${color}`}
          type="button"
          aria-label={color}
          onclick={() => paint(color)}
        ></button>
      {/each}
    </div>
    <form class="flex gap-2" onsubmit={(event) => { event.preventDefault(); const value = friendName.trim(); friendName = ''; if (value) void account.addFriend(value).catch((err) => (note = err instanceof Error ? err.message : 'Could not add them.')); }}>
      <input class="field" placeholder="Friend's name" bind:value={friendName} />
      <button class="btn quiet px-2 py-1" type="submit">Add</button>
    </form>
    {#if account.lobby}
      {#each account.lobby.incoming as person (person.id)}
        <div class="flex items-center justify-between gap-2">
          <span>{person.username} wants to be friends.</span>
          <button class="btn quiet px-2 py-1" type="button" onclick={() => account.acceptFriend(person.id)}>Accept</button>
        </div>
      {/each}
      {#each account.lobby.friends as person (person.id)}
        <p>{person.username}</p>
      {/each}
      {#each account.lobby.requests as request (`${request.tableId}-${request.accountId}`)}
        <div class="flex flex-wrap items-center gap-2">
          <span>{request.username} wants a seat at {request.tableName}.</span>
          <button class="btn quiet px-2 py-1" type="button" onclick={() => account.approve(request.tableId, request.accountId, false)}>Once</button>
          <button class="btn px-2 py-1" type="button" onclick={() => account.approve(request.tableId, request.accountId, true)}>Always</button>
        </div>
      {/each}
    {/if}
  {/if}
  {#if note}<p>{note}</p>{/if}
</div>
