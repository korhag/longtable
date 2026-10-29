<script lang="ts">
  import { fileUrl, loadCatalog, noteCatalogRevision, type CastDef, type CastFrame, type ObjectDef, type Pack, type TileDef } from '../assets/catalog';
  import { editor } from './draft.svelte';
  import { table } from '../table.svelte';

  let packs = $state<Pack[]>([]);
  let tab = $state<'tiles' | 'objects' | 'cast'>('tiles');
  let query = $state('');
  let packId = $state('');
  let error = $state('');
  let selected = $state('');
  let fileInput: HTMLInputElement | undefined = $state();

  const visible = $derived(packs.filter((pack) => !packId || pack.id === packId));
  const tiles = $derived(visible.flatMap((pack) => (pack.tiles ?? []).map((tile) => ({ pack, tile }))));
  const objects = $derived(visible.flatMap((pack) => (pack.objects ?? []).map((object) => ({ pack, object }))));
  const cast = $derived(visible.flatMap((pack) => (pack.cast ?? []).map((actor) => ({ pack, actor }))));

  $effect(() => {
    const revision = table.catalogRevision;
    if (!noteCatalogRevision(revision) && packs.length > 0) return;
    void loadCatalog().then((next) => {
      packs = next;
      void editor.reloadPacks();
    });
  });

  async function importPack(file: File) {
    error = '';
    const body = new FormData();
    body.append('pack', file);
    const response = await fetch('/api/packs/import', {
      method: 'POST',
      headers: { Authorization: `Bearer ${table.sessionToken ?? ''}` },
      body,
    });
    if (!response.ok) {
      error = await response.text();
      return;
    }
    const payload = (await response.json()) as { packs?: Pack[]; warnings?: string[] };
    packs = payload.packs ?? packs;
    error = payload.warnings?.join(' ') ?? '';
    void editor.reloadPacks();
  }

  function matches(name: string, id: string) {
    const needle = query.trim().toLowerCase();
    if (!needle) return true;
    return name.toLowerCase().includes(needle) || id.toLowerCase().includes(needle);
  }

  function castFrame(actor: CastDef, facing: string): CastFrame {
    if (facing === 'east') return actor.views.east;
    if (facing === 'south') return actor.views.south;
    if (facing === 'west') return actor.views.west;
    return actor.views.north;
  }
</script>

<div class="mt-3 text-sm">
  <div class="flex gap-1.5">
    <button class="btn flex-1" type="button" onclick={() => fileInput?.click()}>Import pack</button>
    <input
      bind:this={fileInput}
      class="hidden"
      type="file"
      accept=".zip,application/zip"
      onchange={(event) => {
        const file = event.currentTarget.files?.[0];
        if (file) void importPack(file);
        event.currentTarget.value = '';
      }}
    />
  </div>
  {#if error}
    <p class="mt-2 text-sm text-[var(--color-parchment)]">{error}</p>
  {/if}
  <div class="mt-2 flex gap-1.5">
    <button class="btn quiet flex-1" type="button" class:active={tab === 'tiles'} onclick={() => (tab = 'tiles')}>Tiles</button>
    <button class="btn quiet flex-1" type="button" class:active={tab === 'objects'} onclick={() => (tab = 'objects')}>Objects</button>
    <button class="btn quiet flex-1" type="button" class:active={tab === 'cast'} onclick={() => (tab = 'cast')}>Cast</button>
  </div>
  <div class="mt-2 flex gap-1.5">
    <input class="field" placeholder="Search" bind:value={query} />
    <select class="field w-28" bind:value={packId} aria-label="Pack">
      <option value="">All packs</option>
      {#each packs as pack (pack.id)}
        <option value={pack.id}>{pack.name}</option>
      {/each}
    </select>
  </div>

  <div class="mt-2 flex flex-col gap-1">
    {#if tab === 'tiles'}
      {#each tiles.filter((item) => matches(item.tile.name, item.tile.id)) as item (`${item.pack.id}/${item.tile.id}`)}
        <button class="btn quiet text-left" type="button" onclick={() => (selected = `${item.pack.id}/${item.tile.id}`)}>
          {item.tile.name}
        </button>
        {#if selected === `${item.pack.id}/${item.tile.id}`}
          {@render tileInfo(item.pack, item.tile)}
        {/if}
      {:else}
        <p class="text-sm text-[var(--color-parchment)]/70">No tiles in this pack yet.</p>
      {/each}
    {:else if tab === 'objects'}
      {#each objects.filter((item) => matches(item.object.name, item.object.id)) as item (`${item.pack.id}/${item.object.id}`)}
        <button class="btn quiet text-left" type="button" onclick={() => (selected = `${item.pack.id}/${item.object.id}`)}>
          {item.object.name}
        </button>
        {#if selected === `${item.pack.id}/${item.object.id}`}
          {@render objectInfo(item.pack, item.object)}
        {/if}
      {:else}
        <p class="text-sm text-[var(--color-parchment)]/70">No objects in this pack yet.</p>
      {/each}
    {:else}
      {#each cast.filter((item) => matches(item.actor.name, item.actor.id)) as item (`${item.pack.id}/${item.actor.id}`)}
        <button class="btn quiet text-left" type="button" onclick={() => (selected = `${item.pack.id}/${item.actor.id}`)}>
          {item.actor.name}
        </button>
        {#if selected === `${item.pack.id}/${item.actor.id}`}
          {@render castInfo(item.pack, item.actor)}
        {/if}
      {:else}
        <p class="text-sm text-[var(--color-parchment)]/70">No cast in this pack yet.</p>
      {/each}
    {/if}
  </div>
</div>

{#snippet tileInfo(pack: Pack, tile: TileDef)}
  <div class="px-2 py-1 text-sm text-[var(--color-parchment)]/80">
    <p>{pack.id}/{tile.id} · {tile.kind} · {tile.sourceSizePx[0]}×{tile.sourceSizePx[1]} · {tile.coverageCells[0]} cell</p>
    {#if tile.files[0]}
      <img class="mt-1 h-16 w-16" alt="" src={fileUrl(pack, tile.files[0])} />
    {/if}
  </div>
{/snippet}

{#snippet objectInfo(pack: Pack, object: ObjectDef)}
  <div class="px-2 py-1 text-sm text-[var(--color-parchment)]/80">
    <p>{pack.id}/{object.id} · footprint {object.footprintCells[0]}×{object.footprintCells[1]}</p>
    <p>Movement {object.blocksMovement ? 'blocked' : 'open'} · sight {object.blocksVision ? 'blocked' : 'open'}</p>
    {#each object.renderLayers as layer}
      <p>Pivot {layer.pivotPx[0]}, {layer.pivotPx[1]} on {layer.sizePx[0]}×{layer.sizePx[1]}</p>
      <img class="mt-1 max-h-24" alt="" src={fileUrl(pack, layer.file)} />
    {/each}
  </div>
{/snippet}

{#snippet castInfo(pack: Pack, actor: CastDef)}
  <div class="px-2 py-1 text-sm text-[var(--color-parchment)]/80">
    <p>{pack.id}/{actor.id} · four views · footprint {actor.suggestedFootprintCells?.[0] ?? 1}×{actor.suggestedFootprintCells?.[1] ?? 1}</p>
    <div class="mt-1 flex gap-1">
      {#each ['north', 'east', 'south', 'west'] as facing}
        <img class="max-h-24" alt={facing} src={fileUrl(pack, castFrame(actor, facing).file)} />
      {/each}
    </div>
  </div>
{/snippet}
