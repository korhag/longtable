<script lang="ts">
  import { send } from '../net/socket';
  import { table } from '../table.svelte';
  import { BRUSH_SIZES } from './brush';
  import AssetLibrary from './AssetLibrary.svelte';
  import { editor, type BuildTool } from './draft.svelte';

  let width = $state(30);
  let height = $state(20);
  let blankWidth = $state(30);
  let blankHeight = $state(20);
  let name = $state('New map');
  let fileInput: HTMLInputElement;

  async function importMap(file: File) {
    const body = new FormData();
    body.append('map', file);
    const response = await fetch('/api/maps/import', {
      method: 'POST',
      headers: { Authorization: `Bearer ${table.sessionToken ?? ''}` },
      body,
    });
    if (!response.ok) {
      table.error = await response.text();
      return;
    }
    const payload = await response.json() as { warnings?: string[] };
    const note = payload.warnings?.find((line) => line.trim().length > 0);
    if (note) table.error = note;
  }

  const tools: { id: BuildTool; name: string }[] = [
    { id: 'theme', name: 'Theme' },
    { id: 'terrain', name: 'Terrain' },
    { id: 'wall', name: 'Walls' },
    { id: 'door', name: 'Doors' },
    { id: 'prop', name: 'Props' },
    { id: 'decal', name: 'Decals' },
    { id: 'light', name: 'Lights' },
    { id: 'erase', name: 'Erase' },
  ];

  const lightRanges = [
    { range: 3, name: '3' },
    { range: 6, name: '6' },
    { range: 12, name: '12' },
  ];
  const lightColors = ['#e8a04a', '#f2e2b0', '#d45a3c', '#7eb6e0', '#9d7ad6'];

  const themes = $derived(editor.packs.flatMap((pack) => pack.themes.map((theme) => ({ id: `${pack.id}/${theme.id}`, name: theme.name }))));
  const terrains = $derived(editor.packs.flatMap((pack) => pack.terrains.map((terrain) => ({ id: `${pack.id}/${terrain.id}`, name: terrain.name, swatch: terrain.swatch }))));
  const walls = $derived(editor.packs.flatMap((pack) => pack.walls.map((wall) => ({ id: `${pack.id}/${wall.id}`, name: wall.name }))));
  const doors = $derived(editor.packs.flatMap((pack) => pack.doors.map((door) => ({ id: `${pack.id}/${door.id}`, name: door.name }))));
  const props = $derived(editor.packs.flatMap((pack) => pack.props.map((prop) => ({ id: `${pack.id}/${prop.id}`, name: prop.name }))));
  const decals = $derived(editor.packs.flatMap((pack) => pack.decals.map((decal) => ({ id: `${pack.id}/${decal.id}`, name: decal.name }))));

  function onKey(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return;
    if (event.key === 'r' || event.key === 'R') editor.mirror = !editor.mirror;
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'z') {
      event.preventDefault();
      if (event.shiftKey) editor.redo();
      else editor.undo();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="flex flex-col gap-1 text-sm">
  <AssetLibrary />
  <p class="text-sm text-[var(--color-parchment)]/65">Brush a theme, draw walls, and click a square to place a light. Put the map on the table, then place tokens in Play and drag them. Middle-drag pans. R or right-click mirrors a prop. Erase lifts a wall, a prop, or a light.</p>
  <div class="mt-2 flex gap-1.5">
    <button class="btn quiet flex-1" type="button" onclick={() => fileInput.click()}>Import map</button>
    <input
      bind:this={fileInput}
      class="hidden"
      type="file"
      accept=".dd2vtt,.uvtt,.ltmap,.json,application/json"
      onchange={(event) => {
        const file = event.currentTarget.files?.[0];
        if (file) void importMap(file);
        event.currentTarget.value = '';
      }}
    />
  </div>
  <div class="flex items-end gap-1.5">
    <input class="field w-16" type="number" min="5" max="200" bind:value={blankWidth} aria-label="Blank width" />
    <input class="field w-16" type="number" min="5" max="200" bind:value={blankHeight} aria-label="Blank height" />
    <button class="btn quiet" type="button" onclick={() => send({ type: 'blankMap', width: Math.round(blankWidth), height: Math.round(blankHeight) })}>Blank</button>
  </div>

  <select class="field mt-3" value={editor.draft?.id ?? ''} onchange={(event) => void editor.load(event.currentTarget.value)}>
    <option value="" disabled>Drafts</option>
    {#each editor.summaries as summary (summary.id)}
      <option value={summary.id}>{summary.name}</option>
    {/each}
  </select>
  <div class="mt-2 flex gap-1.5">
    <input class="field w-16" type="number" min="5" max="200" bind:value={width} aria-label="Width" />
    <input class="field w-16" type="number" min="5" max="200" bind:value={height} aria-label="Height" />
    <button class="btn quiet text-sm" type="button" onclick={() => void editor.create(Math.round(width), Math.round(height), name)}>New</button>
  </div>
  <input class="field mt-2" bind:value={name} maxlength="48" aria-label="Draft name" />
  <div class="mt-2 flex gap-1.5">
    <button class="btn quiet flex-1 text-sm" type="button" onclick={() => editor.undo()}>Undo</button>
    <button class="btn quiet flex-1 text-sm" type="button" onclick={() => editor.redo()}>Redo</button>
    <button class="btn quiet flex-1 text-sm" type="button" disabled={!editor.draft} onclick={() => void editor.remove()}>Delete</button>
  </div>

  <div class="mt-3 grid grid-cols-4 gap-1.5">
    {#each tools as item (item.id)}
      <button class="btn quiet text-sm" class:active={editor.tool === item.id} type="button" onclick={() => (editor.tool = item.id)}>{item.name}</button>
    {/each}
  </div>

  {#if editor.tool === 'theme' || editor.tool === 'terrain' || editor.tool === 'erase'}
    <div class="mt-3 flex gap-1.5">
      {#each BRUSH_SIZES as size (size)}
        <button class="btn quiet flex-1 text-sm" class:active={editor.brush === size} type="button" onclick={() => (editor.brush = size)}>{size}</button>
      {/each}
    </div>
  {/if}

  {#if editor.tool === 'theme'}
    <div class="mt-3 flex flex-col gap-1">
      {#each themes as theme (theme.id)}
        <button class="btn quiet text-left text-sm" class:active={editor.themeId === theme.id} type="button" onclick={() => editor.chooseTheme(theme.id)}>{theme.name}</button>
      {/each}
    </div>
  {:else if editor.tool === 'terrain'}
    <div class="mt-3 flex flex-col gap-1">
      {#each terrains as terrain (terrain.id)}
        <button class="btn quiet flex items-center gap-2 text-left text-sm" class:active={editor.terrainId === terrain.id} type="button" onclick={() => (editor.terrainId = terrain.id)}>
          <span class="inline-block h-3 w-3 rounded-full" style={`background:${terrain.swatch}`}></span>{terrain.name}
        </button>
      {/each}
    </div>
  {:else if editor.tool === 'wall'}
    <div class="mt-3 flex flex-col gap-1">
      {#each walls as wall (wall.id)}
        <button class="btn quiet text-left text-sm" class:active={editor.wallId === wall.id} type="button" onclick={() => (editor.wallId = wall.id)}>{wall.name}</button>
      {/each}
    </div>
  {:else if editor.tool === 'door'}
    <div class="mt-3 flex flex-col gap-1">
      {#each doors as door (door.id)}
        <button class="btn quiet text-left text-sm" class:active={editor.doorId === door.id} type="button" onclick={() => (editor.doorId = door.id)}>{door.name}</button>
      {/each}
    </div>
  {:else if editor.tool === 'prop'}
    <div class="mt-3 flex flex-col gap-1">
      {#each props as prop (prop.id)}
        <button class="btn quiet text-left text-sm" class:active={editor.propId === prop.id} type="button" onclick={() => (editor.propId = prop.id)}>{prop.name}</button>
      {/each}
    </div>
  {:else if editor.tool === 'decal'}
    <div class="mt-3 flex flex-col gap-1">
      {#each decals as decal (decal.id)}
        <button class="btn quiet text-left text-sm" class:active={editor.decalId === decal.id} type="button" onclick={() => (editor.decalId = decal.id)}>{decal.name}</button>
      {/each}
    </div>
  {:else if editor.tool === 'light'}
    <p class="mt-3 text-sm text-[var(--color-parchment)]/60">Click a square. The pool is how far the marker reaches, in squares. A second click on the same square updates it.</p>
    <div class="mt-2 flex gap-1.5">
      {#each lightRanges as item (item.range)}
        <button class="btn quiet flex-1 text-sm" class:active={editor.lightRange === item.range} type="button" onclick={() => (editor.lightRange = item.range)}>{item.name}</button>
      {/each}
    </div>
    <div class="mt-2 flex gap-1.5">
      {#each lightColors as color (color)}
        <button
          class="h-8 flex-1 rounded-md"
          style={`background:${color}; outline:${editor.lightColor === color ? '2px solid #f2d48a' : '1px solid transparent'}; outline-offset:2px`}
          type="button"
          aria-label={`Light color ${color}`}
          onclick={() => (editor.lightColor = color)}
        ></button>
      {/each}
    </div>
  {/if}

  <button class="btn mt-3 w-full" type="button" disabled={!editor.draft} onclick={() => void editor.publish()}>Put on the table</button>
  {#if table.scene?.layout}
    <button class="btn quiet mt-1.5 w-full text-sm" type="button" onclick={() => void editor.copyLive()}>Copy the table</button>
  {/if}
  {#if editor.note}
    <p class="mt-2 text-sm text-[var(--color-parchment)]/70">{editor.note}</p>
  {/if}
</div>
