<script lang="ts">
  import { fileUrl, seamCells, type CastDef, type CastFrame, type ObjectDef, type Pack, type TileDef } from '../assets/catalog';
  import { editor } from './draft.svelte';
  import { send } from '../net/socket';
  import { systems } from '../systems/store.svelte';
  import { table } from '../table.svelte';

  let packs = $derived(editor.packs);
  let tab = $state<'tiles' | 'objects' | 'cast'>('tiles');
  let query = $state('');
  let packId = $state('');
  let error = $state('');
  let selected = $state('');
  let fileInput: HTMLInputElement | undefined = $state();
  let checkOnly = false;

  const visible = $derived(packs.filter((pack) => !packId || pack.id === packId));
  const tiles = $derived(visible.flatMap((pack) => (pack.tiles ?? []).map((tile) => ({ pack, tile }))));
  const objects = $derived(visible.flatMap((pack) => (pack.objects ?? []).map((object) => ({ pack, object }))));
  const cast = $derived(visible.flatMap((pack) => (pack.cast ?? []).map((actor) => ({ pack, actor }))));

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
    editor.packs = payload.packs ?? editor.packs;
    error = payload.warnings?.join(' ') ?? '';
  }

  async function checkPack(file: File) {
    error = '';
    const body = new FormData();
    body.append('pack', file);
    const response = await fetch('/api/packs/check', {
      method: 'POST',
      headers: { Authorization: `Bearer ${table.sessionToken ?? ''}` },
      body,
    });
    if (!response.ok) {
      error = await response.text();
      return;
    }
    const report = (await response.json()) as { name: string; tiles: number; objects: number; cast: number; warnings: string[] };
    const ready = `${report.name} is ready: ${report.tiles} tiles, ${report.objects} objects, ${report.cast} figures.`;
    error = [ready, ...report.warnings].join(' ');
  }

  function matches(name: string, id: string) {
    const needle = query.trim().toLowerCase();
    if (!needle) return true;
    return name.toLowerCase().includes(needle) || id.toLowerCase().includes(needle);
  }

  function chooseTile(packId: string, tile: TileDef) {
    const id = `${packId}/${tile.id}`;
    selected = id;
    if (tile.kind === 'overlay') {
      editor.tool = 'decal';
      editor.decalId = id;
      return;
    }
    editor.tool = 'terrain';
    editor.terrainId = id;
  }

  function chooseObject(packId: string, object: ObjectDef) {
    const id = `${packId}/${object.id}`;
    selected = id;
    if (object.kind === 'stairs' || object.placement === 'connection') {
      editor.tool = 'stair';
      editor.stairId = id;
      return;
    }
    if (object.kind === 'door') {
      editor.tool = 'door';
      editor.doorId = id;
      return;
    }
    if (object.kind === 'wall' || object.placement === 'edge') {
      editor.tool = 'wall';
      editor.wallId = id;
      return;
    }
    editor.tool = 'prop';
    editor.propId = id;
  }

  function chooseCast(packId: string, actorId: string) {
    const id = `${packId}/${actorId}`;
    selected = id;
    editor.castId = id;
    const characterId = systems.assigningId;
    if (!characterId) return;
    systems.assigningId = null;
    send({
      type: 'updateCharacter',
      characterId,
      name: null,
      kind: null,
      ownerId: null,
      clearOwner: false,
      numbers: {},
      texts: {},
      setCastAsset: true,
      castAssetId: id,
    });
  }

  const checker = 'background-color:#1c1814;background-image:linear-gradient(45deg,#3a3228 25%,transparent 25%),linear-gradient(-45deg,#3a3228 25%,transparent 25%),linear-gradient(45deg,transparent 75%,#3a3228 75%),linear-gradient(-45deg,transparent 75%,#3a3228 75%);background-size:12px 12px;background-position:0 0,0 6px,6px -6px,-6px 0';

  function groundPartner(pack: Pack, tile: TileDef) {
    for (const candidate of packs) {
      for (const other of candidate.tiles ?? []) {
        if (other.kind !== 'ground') continue;
        if (candidate.id === pack.id && other.id === tile.id) continue;
        return { pack: candidate, tile: other };
      }
    }
    return null;
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
    <button class="btn quiet flex-1" type="button" onclick={() => { checkOnly = true; fileInput?.click(); }}>Check pack</button>
    <button class="btn flex-1" type="button" onclick={() => { checkOnly = false; fileInput?.click(); }}>Import pack</button>
    <input
      bind:this={fileInput}
      class="hidden"
      type="file"
      accept=".zip,application/zip"
      onchange={(event) => {
        const file = event.currentTarget.files?.[0];
        if (file) void (checkOnly ? checkPack(file) : importPack(file));
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
        <button class="btn quiet text-left" type="button" onclick={() => chooseTile(item.pack.id, item.tile)}>
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
        <button class="btn quiet text-left" type="button" onclick={() => chooseObject(item.pack.id, item.object)}>
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
        <button class="btn quiet text-left" type="button" onclick={() => chooseCast(item.pack.id, item.actor.id)}>
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

{#snippet markedPicture(src: string, pivot: [number, number], size: [number, number], label: string)}
  <span class="relative inline-block" style={checker}>
    <img class="block max-h-24" alt={label} src={src} />
    {#if size[0] > 0 && size[1] > 0}
      <span
        class="pointer-events-none absolute h-2 w-2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-[var(--color-gold)]"
        style={`left:${(pivot[0] / size[0]) * 100}%;top:${(pivot[1] / size[1]) * 100}%`}
      ></span>
    {/if}
  </span>
{/snippet}

{#snippet tileInfo(pack: Pack, tile: TileDef)}
  <div class="px-2 py-1 text-sm text-[var(--color-parchment)]/80">
    <p>{pack.id}/{tile.id} · {tile.kind} · {tile.sourceSizePx[0]}×{tile.sourceSizePx[1]} · {tile.coverageCells[0]} cell</p>
    {#if tile.files[0]}
      {@const own = fileUrl(pack, tile.files[0])}
      {#if own}
        <p class="mt-1">Repeat</p>
        <div class="grid w-36 grid-cols-3" style={checker}>
          {#each [0, 1, 2, 3, 4, 5, 6, 7, 8] as cell (cell)}
            <img class="h-12 w-12" alt="" src={own} />
          {/each}
        </div>
        {#if tile.kind === 'ground'}
          {@const partner = groundPartner(pack, tile)}
          {@const neighbor = partner && partner.tile.files[0] ? fileUrl(partner.pack, partner.tile.files[0]) : own}
          <p class="mt-2">{partner ? `Seam with ${partner.tile.name}` : 'This is the only ground, so the seam repeats it.'}</p>
          <div class="grid w-36 grid-cols-3" style={checker}>
            {#each seamCells(own, neighbor ?? own) as src, index (`seam-${index}`)}
              <img class="h-12 w-12" alt="" src={src} />
            {/each}
          </div>
        {/if}
      {/if}
    {/if}
  </div>
{/snippet}

{#snippet objectInfo(pack: Pack, object: ObjectDef)}
  <div class="px-2 py-1 text-sm text-[var(--color-parchment)]/80">
    <p>{pack.id}/{object.id} · footprint {object.footprintCells[0]}×{object.footprintCells[1]}</p>
    <p>Movement {object.blocksMovement ? 'blocked' : 'open'} · sight {object.blocksVision ? 'blocked' : 'open'}</p>
    <p>The gold mark is the pivot. The picture is the fixed-camera art.</p>
    <div class="mt-1 grid w-fit gap-px" style={`grid-template-columns:repeat(${object.footprintCells[0]},1rem)`}>
      {#each Array.from({ length: object.footprintCells[0] * object.footprintCells[1] }, (_, index) => index) as index (index)}
        {@const column = index % object.footprintCells[0]}
        {@const row = Math.floor(index / object.footprintCells[0])}
        {@const root = (object.originCells?.[0] ?? 0) === column && (object.originCells?.[1] ?? 0) === row}
        <span class="h-4 w-4 border border-[var(--color-parchment)]/40" style={root ? 'background:var(--color-gold)' : ''}></span>
      {/each}
    </div>
    {#each object.renderLayers as layer}
      {@const src = fileUrl(pack, layer.file)}
      <p>Pivot {layer.pivotPx[0]}, {layer.pivotPx[1]} on {layer.sizePx[0]}×{layer.sizePx[1]}</p>
      {#if src}
        {@render markedPicture(src, layer.pivotPx, layer.sizePx, object.name)}
      {/if}
    {/each}
  </div>
{/snippet}

{#snippet castInfo(pack: Pack, actor: CastDef)}
  <div class="px-2 py-1 text-sm text-[var(--color-parchment)]/80">
    <p>{pack.id}/{actor.id} · four views · footprint {actor.suggestedFootprintCells?.[0] ?? 1}×{actor.suggestedFootprintCells?.[1] ?? 1}</p>
    <p>The gold mark is the pivot on each fixed-camera view.</p>
    <div class="mt-1 flex gap-1">
      {#each ['north', 'east', 'south', 'west'] as facing}
        {@const frame = castFrame(actor, facing)}
        {@const src = fileUrl(pack, frame.file)}
        {#if src}
          {@render markedPicture(src, frame.pivotPx, frame.sizePx, facing)}
        {/if}
      {/each}
    </div>
  </div>
{/snippet}
