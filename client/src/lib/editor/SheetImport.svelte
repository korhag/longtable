<script lang="ts">
  import { onDestroy } from 'svelte';
  import { cropSheet, framesPerAsset, sheetGrid, sheetManifest, type SheetKind, type SheetSettings } from './sheet';
  import guide from './sheet-guide.md?raw';
  let { onimport }: { onimport: (file: File) => Promise<boolean> } = $props();
  let image = $state<HTMLImageElement | null>(null);
  let url = $state('');
  let busy = $state(false);
  let message = $state('');
  let settings = $state<SheetSettings>({ kind: 'ground', cellWidth: 128, cellHeight: 128, margin: 0, gap: 0, packId: '', name: '', names: '', variants: true, footprintWidth: 1, footprintHeight: 1, pixelsPerWorldUnit: 128, pivotX: 64, pivotY: 127, blocksMovement: true, blocksVision: true });
  const grid = $derived.by(() => { if (!image) return null; try { return sheetGrid(image.naturalWidth, image.naturalHeight, settings); } catch { return null; } });
  const validation = $derived.by(() => { if (!image) return ''; try { const g = sheetGrid(image.naturalWidth, image.naturalHeight, settings); sheetManifest(settings, g.crops.length); return ''; } catch (e) { return (e as Error).message; } });
  const count = $derived(grid ? settings.kind === 'ground' && settings.variants ? 1 : grid.crops.length / framesPerAsset(settings.kind) : 0);
  onDestroy(() => { if (url) URL.revokeObjectURL(url); });
  function preset(kind: SheetKind) {
    settings.kind = kind; settings.cellWidth = kind === 'object' ? 256 : 128;
    settings.cellHeight = kind === 'ground' || kind === 'overlay' ? 128 : kind === 'object' ? 512 : 256;
    settings.pivotX = settings.cellWidth / 2; settings.pivotY = settings.cellHeight - 1;
  }
  async function load(file: File) {
    message = '';
    if (file.type !== 'image/png') { message = 'Choose a PNG sheet with transparency for upright assets.'; return; }
    if (file.size > 32 * 1024 * 1024) { message = 'Keep the PNG sheet below 32 MiB.'; return; }
    const nextUrl = URL.createObjectURL(file); const next = new Image(); next.src = nextUrl;
    try {
      await next.decode();
      if (next.naturalWidth > 8192 || next.naturalHeight > 8192 || next.naturalWidth * next.naturalHeight > 16_777_216) throw new Error('Keep the sheet below 16 megapixels and 8192 pixels per side.');
      if (url) URL.revokeObjectURL(url); url = nextUrl; image = next;
      if (!settings.name) settings.name = file.name.replace(/\.png$/i, '').replace(/[-_]/g, ' ');
      if (!settings.packId) settings.packId = file.name.replace(/\.png$/i, '').toLowerCase().replace(/[^a-z0-9-]+/g, '-').replace(/^-|-$/g, '') || 'my-sheet';
    } catch (e) { URL.revokeObjectURL(nextUrl); message = (e as Error).message; }
  }
  async function install() {
    if (!image || validation || busy) return; busy = true; message = '';
    try { if (await onimport(await cropSheet(image, settings))) message = `Added ${count} ${count === 1 ? 'asset' : 'assets'} to the library. Select them below to start building.`; }
    catch (e) { message = (e as Error).message; } finally { busy = false; }
  }
  function downloadGuide() {
    const href = URL.createObjectURL(new Blob([guide], { type: 'text/markdown' })); const a = document.createElement('a');
    a.href = href; a.download = 'longtable-asset-sheet-guide.md'; a.click(); setTimeout(() => URL.revokeObjectURL(href), 1000);
  }
</script>
<div class="sheet-import">
  <div class="sheet-heading"><strong>From sheet to library</strong><button class="btn quiet" type="button" onclick={downloadGuide}>Asset guide ↓</button></div>
  <p>Upload a regular grid. We cut every cell, name the assets, and add them to your library.</p>
  <fieldset disabled={busy}>
    <label>Asset type<select class="field" value={settings.kind} onchange={e => preset(e.currentTarget.value as SheetKind)}>
      <option value="ground">Ground · top-down tiles</option><option value="overlay">Ground overlays</option><option value="object">Objects · upright props</option><option value="wall">Walls · two axes</option><option value="door">Doors · axes and states</option><option value="cast">Cast · four directions</option>
    </select></label>
    <label class="sheet-file">PNG sprite sheet<input type="file" accept="image/png" onchange={e => { const f = e.currentTarget.files?.[0]; if (f) void load(f); }} /></label>
    {#if settings.kind === 'cast'}<p>Four columns: North, East, South, West. Each row is one character.</p>{:else if settings.kind === 'wall'}<p>Two columns: horizontal edge, vertical edge. Each row is one wall style.</p>{:else if settings.kind === 'door'}<p>Four columns: horizontal closed, horizontal open, vertical closed, vertical open. Each row is one door.</p>{/if}
    <div class="sheet-fields">
      <label>Cell width<input class="field" type="number" min="1" max="2048" bind:value={settings.cellWidth} /></label><label>Cell height<input class="field" type="number" min="1" max="2048" bind:value={settings.cellHeight} /></label>
      <label>Outer margin<input class="field" type="number" min="0" bind:value={settings.margin} /></label><label>Gutter<input class="field" type="number" min="0" bind:value={settings.gap} /></label>
    </div>
    {#if image}
      <div class="sheet-preview"><img src={url} alt="Sheet preview with crop boundaries" />{#if grid}{#each grid.crops as crop, i}<span style={`left:${crop.x / image.naturalWidth * 100}%;top:${crop.y / image.naturalHeight * 100}%;width:${crop.width / image.naturalWidth * 100}%;height:${crop.height / image.naturalHeight * 100}%`}><b>{i + 1}</b></span>{/each}{/if}</div>
      <p>{image.naturalWidth} × {image.naturalHeight} px{grid ? ` · ${grid.columns} columns × ${grid.rows} rows · ${count} assets` : ''}</p>
    {/if}
    <label>Pack name<input class="field" placeholder="Ancient woodland" bind:value={settings.name} /></label><label>Pack ID<input class="field" placeholder="ancient-woodland" bind:value={settings.packId} /></label>
    {#if settings.kind === 'ground'}<label class="sheet-checkbox"><input type="checkbox" bind:checked={settings.variants} /> One terrain with natural variants</label><p>Enable for several versions of the same surface. Disable for different terrain materials.</p>{/if}
    <label>Asset names <small>Optional · one per asset, in reading order</small><textarea class="field" rows="3" placeholder={settings.kind === 'cast' ? 'Human ranger\nElf scout\nDwarf warrior\nOrc guardian' : 'Leave blank to name automatically'} bind:value={settings.names}></textarea></label>
    {#if !['ground', 'overlay'].includes(settings.kind)}
      <details><summary>Size, foot pivot & collision</summary><div class="sheet-fields">
        <label>Footprint width<input class="field" type="number" min="1" max="8" bind:value={settings.footprintWidth} /></label><label>Footprint depth<input class="field" type="number" min="1" max="8" bind:value={settings.footprintHeight} /></label>
        <label>Pivot X (px)<input class="field" type="number" min="0" bind:value={settings.pivotX} /></label><label>Pivot Y (px)<input class="field" type="number" min="0" bind:value={settings.pivotY} /></label>
      </div><label>Pixels per world unit<input class="field" type="number" min="1" max="4096" bind:value={settings.pixelsPerWorldUnit} /></label><p>The pivot is where feet or roots touch the ground. Keep it consistent across every frame.</p>
      {#if settings.kind !== 'cast'}<label class="sheet-checkbox"><input type="checkbox" bind:checked={settings.blocksMovement} /> Blocks movement</label><label class="sheet-checkbox"><input type="checkbox" bind:checked={settings.blocksVision} /> Blocks sight</label>{/if}</details>
    {/if}
  </fieldset>
  {#if validation}<p class="sheet-notice" role="status">{validation}</p>{/if}{#if message}<p class="sheet-notice" role="status">{message}</p>{/if}
  <button class="btn primary" type="button" disabled={!image || !!validation || busy} onclick={install}>{busy ? 'Cutting and importing…' : `Add ${count || ''} assets to library`}</button>
  <p class="sheet-note">Use a fresh pack ID for a new version. Existing assets are preserved. Labels, borders, and backgrounds in a source sheet become part of its pictures.</p>
</div>
<style>
  .sheet-import { border:1px solid var(--color-gold,#b99b65); border-radius:12px; padding:12px; background:#141714; display:grid; gap:10px; }
  .sheet-heading { display:flex; align-items:center; justify-content:space-between; gap:8px; }
  .sheet-heading strong { color:var(--color-gold); font-size:14px; }
  p { font-size:12px; line-height:1.5; opacity:.78; margin:0; }
  fieldset { border:0; padding:0; min-width:0; display:grid; gap:10px; }
  label { display:grid; gap:5px; font-size:12px; min-width:0; }
  .sheet-fields { display:grid; grid-template-columns:1fr 1fr; gap:8px; }
  .sheet-fields input,.field { width:100%; min-width:0; }
  .sheet-checkbox { display:flex; align-items:center; gap:8px; }
  .sheet-preview { position:relative; background:repeating-conic-gradient(#242722 0% 25%,#171a17 0% 50%) 0 0 / 16px 16px; }
  .sheet-preview img { display:block; width:100%; }
  .sheet-preview span { position:absolute; border:1px solid #d6ba78aa; pointer-events:none; }
  .sheet-preview b { font:10px sans-serif; background:#10130ecc; padding:1px 3px; color:#e4c994; }
  .sheet-notice { color:var(--color-gold); opacity:1; }
  .sheet-note { opacity:.6; }
  small { opacity:.65; font-size:11px; }
  details { border-top:1px solid #b99b6533; padding-top:8px; }
  summary { cursor:pointer; font-size:12px; margin-bottom:8px; }
  .sheet-file input { max-width:100%; font-size:12px; }
</style>
