<script lang="ts">
  import { send } from '../net/socket';
  import { table } from '../table.svelte';

  const token = $derived(table.tokens.find((item) => item.id === table.selectedTokenId) ?? null);
  const gm = $derived(table.role === 'gm' && !table.viewAs);
  const colors = ['#d4a656', '#c45c4a', '#6e9a72', '#6f8fbf', '#8d5b86', '#e7d7b6'];

  async function portrait(file: File, id: string) {
    const body = new FormData();
    body.append('image', file);
    const response = await fetch(`/api/tokens/${id}/image`, {
      method: 'POST',
      headers: { Authorization: `Bearer ${table.sessionToken ?? ''}` },
      body,
    });
    if (!response.ok) table.error = await response.text();
  }
</script>

{#if token}
  <section class="glass pointer-events-auto w-full rounded-2xl p-3">
    <p class="display text-lg">{token.name}</p>
    <p class="text-sm text-[var(--color-parchment)]/60">
      {token.hidden ? 'Hidden from players' : 'On the table'} · sight {token.visionRadius}
    </p>

    {#if gm}
      <label class="mt-3 block text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">Name
        <input
          class="field mt-1"
          value={token.name}
          maxlength="24"
          onchange={(event) =>
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: event.currentTarget.value,
              ownerId: null,
              clearOwner: false,
              hidden: null,
              visionRadius: null,
              grantsVision: null,
              color: null,
              characterId: null,
              clearCharacter: false,
            })}
        />
      </label>
      <label class="mt-2 block text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">Owner
        <select
          class="field mt-1"
          value={token.ownerId ?? ''}
          onchange={(event) => {
            const ownerId = event.currentTarget.value;
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: null,
              ownerId: ownerId || null,
              clearOwner: ownerId === '',
              hidden: null,
              visionRadius: null,
              grantsVision: null,
              color: null,
              characterId: null,
              clearCharacter: false,
            });
          }}
        >
          <option value="">No one</option>
          {#each table.players as player (player.id)}
            <option value={player.id}>{player.name}</option>
          {/each}
        </select>
      </label>
      <label class="mt-2 block text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">Character
        <select
          class="field mt-1"
          value={token.characterId ?? ''}
          onchange={(event) => {
            const characterId = event.currentTarget.value;
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: null,
              ownerId: null,
              clearOwner: false,
              hidden: null,
              visionRadius: null,
              grantsVision: null,
              color: null,
              characterId: characterId || null,
              clearCharacter: characterId === '',
            });
          }}
        >
          <option value="">None</option>
          {#each table.characters as character (character.id)}
            <option value={character.id}>{character.name}</option>
          {/each}
        </select>
      </label>
      <label class="mt-2 block text-sm tracking-wide text-[var(--color-gold)]/70 uppercase">Vision, in squares
        <input
          class="field mt-1"
          type="number"
          min="0"
          max="80"
          value={token.visionRadius}
          onchange={(event) =>
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: null,
              ownerId: null,
              clearOwner: false,
              hidden: null,
              visionRadius: Number(event.currentTarget.value),
              grantsVision: null,
              color: null,
              characterId: null,
              clearCharacter: false,
            })}
        />
      </label>
      <div class="mt-2 flex gap-1.5">
        {#each colors as color (color)}
          <button
            class="h-5 w-5 rounded-full border border-white/30"
            style={`background:${color}`}
            type="button"
            aria-label={color}
            onclick={() =>
              send({
                type: 'updateToken',
                tokenId: token.id,
                name: null,
                ownerId: null,
                clearOwner: false,
                hidden: null,
                visionRadius: null,
                grantsVision: null,
                color,
                characterId: null,
                clearCharacter: false,
              })}
          ></button>
        {/each}
      </div>
      <div class="mt-3 flex flex-wrap gap-2 text-sm">
        <button
          class="btn quiet"
          type="button"
          onclick={() =>
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
            })}
        >{token.hidden ? 'Show' : 'Hide'}</button>
        <button
          class="btn quiet"
          type="button"
          onclick={() =>
            send({
              type: 'updateToken',
              tokenId: token.id,
              name: null,
              ownerId: null,
              clearOwner: false,
              hidden: null,
              visionRadius: null,
              grantsVision: !token.grantsVision,
              color: null,
              characterId: null,
              clearCharacter: false,
            })}
        >{token.grantsVision ? 'Sight on' : 'Sight off'}</button>
        <button class="btn quiet" type="button" onclick={() => send({ type: 'deleteToken', tokenId: token.id })}>Remove</button>
      </div>
    {/if}

    {#if gm || token.ownerId === table.playerId}
      <label class="btn quiet mt-3 block text-center text-sm">
        Portrait
        <input
          class="hidden"
          type="file"
          accept="image/png,image/jpeg,image/webp"
          onchange={(event) => {
            const file = event.currentTarget.files?.[0];
            if (file) void portrait(file, token.id);
            event.currentTarget.value = '';
          }}
        />
      </label>
    {/if}
  </section>
{/if}
