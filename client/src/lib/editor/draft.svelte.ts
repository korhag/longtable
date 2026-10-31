// ------------------------------------------------------------
// Module: draft
// Purpose: The GM's map draft. Strokes are undone as a whole,
//          and the draft is saved shortly after the last change.
// ------------------------------------------------------------

import { findIn, loadCatalog, type Pack } from '../assets/catalog';
import type { EnvironmentLighting } from '../net/bindings/EnvironmentLighting';
import type { LightKind } from '../net/bindings/LightKind';
import { DAYLIGHT, LIGHT_PRESETS } from '../lightingPresets';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { PlacedDecal } from '../net/bindings/PlacedDecal.ts';
import type { PlacedProp } from '../net/bindings/PlacedProp.ts';
import { table } from '../table.svelte';
import { brushCells, floodCells, wallEdges, nearestEdge, type BrushSize, type Cell } from './brush';
import { propMirror } from './placementRepair';
import { clearVariantPins, pinVariant } from './variants';
import { decodeCells, encodeCells } from './cells';
import { cellLevel } from '../render/levels';
import { blocksDoor, fillTheme, footprintsOverlap, type ScatterEntry } from './scatter';

export type BuildTool = 'theme' | 'terrain' | 'wall' | 'door' | 'prop' | 'decal' | 'light' | 'erase' | 'raise' | 'stair';

export interface DraftSummary {
  id: string;
  name: string;
  updatedAt: number;
  width: number;
  height: number;
}

export interface Draft {
  id: string;
  name: string;
  updatedAt: number;
  sceneId?: string | null;
  /** Ambient settings when copied or last published; retained across draft reloads. */
  environmentBase?: EnvironmentLighting | null;
  layout: MapLayout;
}

class EditorStore {
  active = $state(false);
  tool = $state<BuildTool>('theme');
  brush = $state<BrushSize>(3);
  paintMode = $state<'brush' | 'fill'>('brush');
  wallMode = $state<'freehand' | 'line' | 'room'>('line');
  themeId = $state('core/grassland');
  terrainId = $state('core/grass');
  wallId = $state('core/hedge');
  doorId = $state('core/hedge-gate');
  propId = $state('core/oak');
  decalId = $state('core/tuft');
  stairId = $state('');
  tier = $state<0 | 1 | 2>(1);
  castId = $state('');
  lightRange = $state(5);
  lightColor = $state('#ffad42');
  lightKind = $state<LightKind>('torch');
  lightIntensity = $state(1.4);
  private environmentBase: MapLayout | null = null;
  mirror = $state(false);
  packs = $state<Pack[]>([]);
  summaries = $state<DraftSummary[]>([]);
  draft = $state<Draft | null>(null);
  ghost = $state<Cell[]>([]);
  note = $state('');
  private past: MapLayout[] = [];
  private future: MapLayout[] = [];
  private timer = 0;
  private saving: Promise<boolean> | null = null;
  private dirty = false;
  private stroke: Cell[] = [];
  private base: MapLayout | null = null;
  private painting = false;
  private startPoint: Cell | null = null;
  private lastPoint: Cell | null = null;

  async reloadPacks() {
    this.packs = await loadCatalog();
  }

  async enter() {
    this.active = true;
    this.packs = await loadCatalog();
    await this.refresh();
    const first = this.summaries[0];
    if (!this.draft && first) await this.load(first.id);
  }

  leave() {
    this.active = false;
    this.ghost = [];
    this.painting = false;
  }

  async refresh() {
    const response = await fetch('/api/drafts', { headers: this.headers() });
    if (response.ok) this.summaries = (await response.json()) as DraftSummary[];
  }

  async create(width: number, height: number, name: string) {
    const response = await fetch('/api/drafts', {
      method: 'POST',
      headers: this.headers(),
      body: JSON.stringify({ name, width, height, copyLive: false }),
    });
    if (!response.ok) {
      this.note = await response.text();
      return;
    }
    this.draft = normalize((await response.json()) as Draft);
    this.past = [];
    this.future = [];
    this.environmentBase = null;
    await this.refresh();
  }

  async loadForestSample() {
    if (this.draft && !(await this.save())) return;
    try {
      const response = await fetch('/api/samples/forest', { method: 'POST', headers: this.headers() });
      if (!response.ok) { this.note = await response.text(); return; }
      this.draft = normalize((await response.json()) as Draft);
      this.environmentBase = null;
      this.past = [];
      this.future = [];
      this.ghost = [];
      this.chooseTheme('forest/woodland');
      this.stairId = 'forest/stairs';
      this.note = 'The Old Road is on the table: four adventurers, four bandits, a cabin, and three connected heights. Switch to Play to explore.';
      await this.refresh();
    } catch { this.note = 'The forest encounter could not be loaded. Check the server connection.'; }
  }

  fillMap() {
    if (!this.draft) return;
    this.base = cloneLayout(this.draft.layout);
    this.stroke = Array.from({ length: this.draft.layout.width * this.draft.layout.height }, (_, index) => ({ x: index % this.draft!.layout.width, y: Math.floor(index / this.draft!.layout.width) }));
    if (this.tool === 'terrain') this.draft.layout = paintTerrain(this.base, this.stroke, this.terrainId);
    this.finish();
    this.base = null;
    this.stroke = [];
  }

  async copyLive() {
    const response = await fetch('/api/drafts', {
      method: 'POST',
      headers: this.headers(),
      body: JSON.stringify({ name: 'Copy', copyLive: true }),
    });
    if (!response.ok) {
      this.note = await response.text();
      return;
    }
    this.draft = normalize((await response.json()) as Draft);
    this.environmentBase = null;
    await this.refresh();
  }

  async load(id: string) {
    const response = await fetch(`/api/drafts/${id}`, { headers: this.headers() });
    if (!response.ok) return;
    this.draft = normalize((await response.json()) as Draft);
    this.past = [];
    this.future = [];
    this.environmentBase = null;
    this.ghost = [];
  }

  async remove() {
    if (!this.draft) return;
    await fetch(`/api/drafts/${this.draft.id}`, { method: 'DELETE', headers: this.headers() });
    this.draft = null;
    await this.refresh();
  }

  async publish() {
    if (!this.draft) return;
    // A live day/night change wins over a draft that did not edit lighting.
    const currentEnvironment = this.draft.layout.environment ?? DAYLIGHT;
    const baseEnvironment = this.draft.environmentBase ?? DAYLIGHT;
    if (sameEnvironment(currentEnvironment, baseEnvironment) && this.draft.sceneId === table.scene?.id) {
      this.draft.layout.environment = { ...(table.scene?.environment ?? DAYLIGHT) };
      this.draft.environmentBase = { ...this.draft.layout.environment };
    }
    window.clearTimeout(this.timer);
    const saved = await this.save();
    if (!saved) return;
    const response = await fetch(`/api/drafts/${this.draft.id}/publish`, { method: 'POST', headers: this.headers() });
    if (!response.ok) {
      this.note = await response.text();
      return;
    }
    const fresh = await fetch(`/api/drafts/${this.draft.id}`, { headers: this.headers() });
    if (fresh.ok) this.draft = normalize((await fresh.json()) as Draft);
    this.draft.environmentBase = { ...(this.draft.layout.environment ?? DAYLIGHT) };
    this.environmentBase = null;
    this.note = 'On the table';
  }

  undo() {
    if (!this.draft || this.past.length === 0 || this.painting) return;
    this.future.push(cloneLayout(this.draft.layout));
    this.draft.layout = this.past.pop() ?? this.draft.layout;
    this.schedule();
  }

  redo() {
    if (!this.draft || this.future.length === 0 || this.painting) return;
    this.past.push(cloneLayout(this.draft.layout));
    this.draft.layout = this.future.pop() ?? this.draft.layout;
    this.schedule();
  }

  chooseLight(kind: LightKind) {
    const preset = LIGHT_PRESETS.find(item => item.kind === kind)!;
    this.lightKind = kind;
    this.lightColor = preset.color;
    this.lightRange = preset.range;
    this.lightIntensity = preset.intensity;
    this.tool = 'light';
  }

  setEnvironment(environment: EnvironmentLighting, commit = true) {
    if (!this.draft) return;
    this.draft.environmentBase ??= { ...(this.draft.layout.environment ?? DAYLIGHT) };
    this.environmentBase ??= cloneLayout(this.draft.layout);
    this.draft.layout.environment = { ...environment };
    if (!commit) return;
    if (JSON.stringify(this.environmentBase) !== JSON.stringify(this.draft.layout)) {
      this.past.push(this.environmentBase);
      if (this.past.length > 40) this.past.shift();
      this.future = [];
      this.schedule();
    }
    this.environmentBase = null;
  }

  syncLiveEnvironment(sceneId: string, environment: EnvironmentLighting) {
    if (!this.draft || this.draft.sceneId !== sceneId) return;
    this.draft.layout.environment = { ...environment };
    this.draft.environmentBase = { ...environment };
    // An unrelated undo must not restore the live scene's earlier time of day.
    for (const past of [...this.past, ...this.future]) past.environment = { ...environment };
    this.environmentBase = null;
    this.schedule();
  }

  chooseTheme(id: string) {
    this.themeId = id;
    this.tool = 'theme';
    const theme = findIn(this.packs, id, (pack) => pack.themes);
    if (!theme) return;
    this.terrainId = `${theme.pack.id}/${theme.def.terrain}`;
    if (theme.def.wall) this.wallId = `${theme.pack.id}/${theme.def.wall}`;
    if (theme.def.door) this.doorId = `${theme.pack.id}/${theme.def.door}`;
  }

  pointer(phase: 'down' | 'move' | 'up' | 'hover', point: { x: number; y: number }, button: number) {
    const current = this.draft?.layout;
    if (!current) return;
    if (phase === 'hover') {
      if (this.paints()) this.ghost = brushCells(point.x, point.y, this.brush, current.width, current.height);
      else if (this.tool === 'light') this.ghost = lightCell(point.x, point.y, current.width, current.height);
      else if (this.tool === 'stair') this.ghost = stairCells(current, point.x, point.y);
      else if (this.tool === 'prop') {
        const [widthCells, depthCells] = footprintOf(this.packs, this.propId);
        this.ghost = propCells(point.x, point.y, widthCells, depthCells, current.width, current.height);
      } else this.ghost = [];
      return;
    }
    // PointerEvent.button is -1 during movement; the down event owns this stroke.
    if (phase === 'move' && this.painting) button = 0;
    if (button === 2) {
      if (phase === 'down' && this.tool === 'prop') this.mirror = !this.mirror;
      if (phase === 'down' && this.tool === 'terrain') this.pinPicture(point);
      return;
    }
    if (button !== 0) return;
    if (phase === 'down') {
      this.base = cloneLayout(current);
      this.stroke = [];
      this.painting = true;
      this.startPoint = { ...point };
      this.lastPoint = null;
      this.step(point);
    } else if (phase === 'move' && this.painting) {
      this.step(point);
    } else if (phase === 'up' && this.painting) {
      this.step(point);
      this.finish();
      this.painting = false;
      this.base = null;
      this.stroke = [];
      this.startPoint = null;
      this.lastPoint = null;
    }
  }

  private pinPicture(point: { x: number; y: number }) {
    const layout = this.draft?.layout;
    if (!layout || !this.draft) return;
    const x = Math.floor(point.x);
    const y = Math.floor(point.y);
    if (x < 0 || y < 0 || x >= layout.width || y >= layout.height) return;
    const bytes = decodeCells(layout.cells);
    const asset = layout.terrains[bytes[y * layout.width + x] ?? 0] ?? '';
    const tile = findIn(this.packs, asset, (pack) => pack.tiles ?? []);
    const count = tile?.def.files.length ?? 1;
    const name = tile?.def.name ?? 'This tile';
    if (count <= 1) {
      this.note = `${name} has one picture.`;
      return;
    }
    const pinned = pinVariant(layout, x, y, asset, count);
    this.past.push(cloneLayout(layout));
    if (this.past.length > 40) this.past.shift();
    this.future = [];
    this.draft.layout = pinned.layout;
    this.note = pinned.stored === 0 ? `${name} on this square follows the map.` : `${name} on this square keeps picture ${pinned.shown}.`;
    this.schedule();
  }

  private paints() {
    return this.tool === 'theme' || this.tool === 'terrain' || this.tool === 'erase' || this.tool === 'raise';
  }

  private step(point: { x: number; y: number }) {
    const layout = this.draft?.layout;
    const base = this.base;
    if (!layout || !base || !this.draft) return;
    if (this.paints()) {
      const bucket = this.paintMode === 'fill' && (this.tool === 'theme' || this.tool === 'terrain');
      if (bucket && this.stroke.length > 0) return;
      const cells = bucket
        ? floodCells(point.x, point.y, base.width, base.height, decodeCells(base.cells), base.levels ? decodeCells(base.levels) : undefined)
        : this.strokeCells(point, layout);
      const marked = new Set(this.stroke.map((cell) => cell.y * layout.width + cell.x));
      for (const cell of cells) {
        if (!marked.has(cell.y * layout.width + cell.x)) { this.stroke.push(cell); marked.add(cell.y * layout.width + cell.x); }
      }
      this.lastPoint = { ...point };
      this.ghost = this.stroke;
      if (this.tool === 'terrain') this.draft.layout = paintTerrain(base, this.stroke, this.terrainId);
      if (this.tool === 'raise') this.draft.layout = paintLevels(base, this.stroke, this.tier);
      return;
    }
    if (this.tool === 'wall') {
      if (this.wallMode === 'freehand') {
        const start = this.lastPoint ?? point;
        const steps = Math.max(1, Math.ceil(Math.hypot(point.x - start.x, point.y - start.y) * 3));
        for (let i = 0; i <= steps; i += 1) {
          const edge = nearestEdge(start.x + (point.x - start.x) * i / steps, start.y + (point.y - start.y) * i / steps, layout.width, layout.height);
          if (edge) this.draft.layout = addPiece(this.draft.layout, edge, 'wall', this.wallId);
        }
      } else {
        const edges = wallEdges(this.startPoint ?? point, point, this.wallMode, layout.width, layout.height);
        this.draft.layout = edges.reduce((current, edge) => addPiece(current, edge, 'wall', this.wallId), base);
      }
      this.lastPoint = { ...point };
      return;
    }
    if (this.stroke.length > 0) return;
    this.stroke.push({ x: Math.floor(point.x), y: Math.floor(point.y) });
    if (this.tool === 'stair') {
      const edge = nearestEdge(point.x, point.y, layout.width, layout.height);
      if (!edge) return;
      const placed = addStair(layout, edge, this.stairId || 'stair');
      this.draft.layout = placed.layout;
      this.note = placed.note;
    } else if (this.tool === 'door') {
      const edge = nearestEdge(point.x, point.y, layout.width, layout.height);
      if (edge) this.draft.layout = addPiece(layout, edge, 'door', this.doorId);
    } else if (this.tool === 'prop') {
      const placed = addProp(layout, Math.floor(point.x), Math.floor(point.y), this.propId, this.mirror, this.packs);
      this.draft.layout = placed.layout;
      if (placed.note) this.note = placed.note;
    } else if (this.tool === 'decal') {
      this.draft.layout = addDecal(layout, point.x, point.y, this.decalId, this.packs);
    } else if (this.tool === 'light') {
      this.draft.layout = addLight(layout, point.x, point.y, this.lightRange, this.lightColor, this.lightKind, this.lightIntensity);
    }
  }

  private strokeCells(point: Cell, layout: MapLayout): Cell[] {
    const start = this.lastPoint ?? point;
    const steps = Math.max(1, Math.ceil(Math.hypot(point.x - start.x, point.y - start.y) * 2));
    const cells: Cell[] = [];
    for (let i = 0; i <= steps; i += 1) {
      cells.push(...brushCells(start.x + (point.x - start.x) * i / steps, start.y + (point.y - start.y) * i / steps, this.brush, layout.width, layout.height));
    }
    return cells;
  }

  private finish() {
    if (!this.draft || !this.base) return;
    let next = this.draft.layout;
    if (this.tool === 'theme') {
      const theme = findIn(this.packs, this.themeId, (pack) => pack.themes);
      const terrain = theme ? `${theme.pack.id}/${theme.def.terrain}` : this.terrainId;
      next = fillTheme(this.base, this.stroke, terrain, entriesFor(this.packs, this.themeId), Math.floor(Math.random() * 1_000_000_000), (asset) => footprintOf(this.packs, asset));
    } else if (this.tool === 'erase') {
      next = erase(this.base, this.stroke, this.packs);
    }
    if (JSON.stringify(next) === JSON.stringify(this.base)) return;
    this.past.push(this.base);
    if (this.past.length > 40) this.past.shift();
    this.future = [];
    this.draft.layout = next;
    this.schedule();
  }

  private schedule() {
    window.clearTimeout(this.timer);
    this.timer = window.setTimeout(() => void this.save(), 800);
  }

  private async save(): Promise<boolean> {
    if (!this.draft) return false;
    if (this.saving) {
      this.dirty = true;
      const ok = await this.saving;
      if (!ok) return false;
      return this.dirty ? this.save() : true;
    }
    const id = this.draft.id;
    const body = JSON.stringify(this.draft);
    this.dirty = false;
    this.saving = this.postDraft(id, body);
    const ok = await this.saving;
    this.saving = null;
    if (ok && this.dirty) return this.save();
    return ok;
  }

  private async postDraft(id: string, body: string): Promise<boolean> {
    try {
      const response = await fetch(`/api/drafts/${id}`, { method: 'PUT', headers: this.headers(), body });
      if (!response.ok) {
        this.note = await response.text();
        return false;
      }
      const saved = (await response.json()) as { updatedAt?: number };
      if (this.draft?.id === id && typeof saved.updatedAt === 'number') this.draft.updatedAt = saved.updatedAt;
      return true;
    } catch {
      this.note = 'The map could not be saved.';
      return false;
    }
  }

  private headers(): Record<string, string> {
    return {
      Authorization: `Bearer ${table.sessionToken ?? ''}`,
      'Content-Type': 'application/json',
    };
  }
}

// A Svelte state proxy cannot be structuredClone'd. The layout is plain data.
function cloneLayout(layout: MapLayout): MapLayout {
  return JSON.parse(JSON.stringify(layout)) as MapLayout;
}

function paintTerrain(base: MapLayout, cells: Cell[], terrain: string): MapLayout {
  const next = cloneLayout(base);
  let index = next.terrains.indexOf(terrain);
  if (index < 0) {
    next.terrains.push(terrain);
    index = next.terrains.length - 1;
  }
  const bytes = decodeCells(next.cells);
  for (const cell of cells) {
    const offset = cell.y * next.width + cell.x;
    if (offset >= 0 && offset < bytes.length) bytes[offset] = index;
  }
  next.cells = encodeCells(bytes);
  return clearVariantPins(next, cells);
}

function addPiece(layout: MapLayout, edge: { x: number; y: number; axis: 'h' | 'v' }, kind: 'wall' | 'door', asset: string): MapLayout {
  const next = cloneLayout(layout);
  const index = next.pieces.findIndex((piece) => piece.x === edge.x && piece.y === edge.y && piece.axis === edge.axis);
  if (index >= 0) {
    const existing = next.pieces[index];
    if (!existing || (existing.kind === kind && existing.asset === asset) || (kind === 'wall' && existing.kind === 'door')) return layout;
    next.pieces[index] = { ...existing, kind, asset, open: false };
    return next;
  }
  next.pieces.push({ id: crypto.randomUUID(), kind, asset, x: edge.x, y: edge.y, axis: edge.axis, open: false });
  return next;
}

/** The squares a prop will cover. The click uses the top-left of the footprint, matching placement. */
export function propCells(x: number, y: number, widthCells: number, depthCells: number, mapWidth: number, mapHeight: number): Cell[] {
  const originX = Math.floor(x);
  const originY = Math.floor(y);
  const spanX = Math.max(1, Math.floor(widthCells));
  const spanY = Math.max(1, Math.floor(depthCells));
  const cells: Cell[] = [];
  for (let stepY = 0; stepY < spanY; stepY += 1) {
    for (let stepX = 0; stepX < spanX; stepX += 1) {
      const cellX = originX + stepX;
      const cellY = originY + stepY;
      if (cellX < 0 || cellY < 0 || cellX >= mapWidth || cellY >= mapHeight) continue;
      cells.push({ x: cellX, y: cellY });
    }
  }
  return cells;
}

function addProp(layout: MapLayout, x: number, y: number, asset: string, mirror: boolean, packs: Pack[]): { layout: MapLayout; note: string | null } {
  const [w, d] = footprintOf(packs, asset);
  const foot = { x, y, w, d };
  if (x < 0 || y < 0 || x + w > layout.width || y + d > layout.height) return { layout, note: null };
  if (blocksDoor(foot, layout)) return { layout, note: null };
  const busy = layout.props.some((prop) => footprintsOverlap(foot, { x: prop.x, y: prop.y, ...sizeOf(packs, prop.asset) }));
  if (busy) return { layout, note: null };
  const flip = propMirror(asset, packs, mirror);
  const placed: PlacedProp = { id: crypto.randomUUID(), asset, x, y, mirror: flip.mirror };
  return { layout: { ...layout, props: [...layout.props, placed] }, note: flip.note };
}

function addDecal(layout: MapLayout, x: number, y: number, asset: string, packs: Pack[]): MapLayout {
  const found = findIn(packs, asset, (pack) => pack.decals);
  const decal: PlacedDecal = {
    asset,
    x,
    y,
    scale: 1,
    angle: found?.def.rotate ? Math.floor(Math.random() * 360) : 0,
  };
  return { ...layout, decals: [...layout.decals, decal] };
}

function erase(base: MapLayout, cells: Cell[], packs: Pack[]): MapLayout {
  const next = cloneLayout(base);
  const touches = (x: number, y: number) => cells.some((cell) => cell.x === x && cell.y === y);
  next.props = next.props.filter((prop) => {
    const { w, d } = sizeOf(packs, prop.asset);
    return !cells.some((cell) => cell.x >= prop.x && cell.x < prop.x + w && cell.y >= prop.y && cell.y < prop.y + d);
  });
  next.decals = next.decals.filter((decal) => !touches(Math.floor(decal.x), Math.floor(decal.y)));
  next.pieces = next.pieces.filter((piece) => {
    if (piece.axis === 'h') return !touches(piece.x, piece.y) && !touches(piece.x, piece.y - 1);
    return !touches(piece.x, piece.y) && !touches(piece.x - 1, piece.y);
  });
  next.lights = (next.lights ?? []).filter((light) => !touches(Math.floor(light.x), Math.floor(light.y)));
  if (next.stairs) next.stairs = next.stairs.filter((stair) => !touches(stair.x, stair.y) && !touches(stair.x2, stair.y2));
  return next;
}

function paintLevels(base: MapLayout, cells: Cell[], tier: number): MapLayout {
  const next = cloneLayout(base);
  const count = next.width * next.height;
  const decoded = next.levels ? decodeCells(next.levels) : new Uint8Array(count);
  const grid = decoded.length === count ? decoded : new Uint8Array(count);
  for (const cell of cells) {
    const index = cell.y * next.width + cell.x;
    if (index >= 0 && index < grid.length) grid[index] = tier;
  }
  next.levels = encodeCells(grid);
  return next;
}

function stairCells(layout: MapLayout, x: number, y: number): Cell[] {
  const edge = nearestEdge(x, y, layout.width, layout.height);
  return edge ? (edgePair(layout, edge) ?? []) : [];
}

function edgePair(layout: MapLayout, edge: { x: number; y: number; axis: 'h' | 'v' }): [Cell, Cell] | null {
  if (edge.axis === 'h') {
    if (edge.y <= 0 || edge.y >= layout.height) return null;
    return [
      { x: edge.x, y: edge.y },
      { x: edge.x, y: edge.y - 1 },
    ];
  }
  if (edge.x <= 0 || edge.x >= layout.width) return null;
  return [
    { x: edge.x, y: edge.y },
    { x: edge.x - 1, y: edge.y },
  ];
}

function addStair(layout: MapLayout, edge: { x: number; y: number; axis: 'h' | 'v' }, asset: string): { layout: MapLayout; note: string } {
  const pair = edgePair(layout, edge);
  if (!pair) return { layout, note: 'That edge is on the border of the map.' };
  const [a, b] = pair;
  if (Math.abs(cellLevel(layout, a.x, a.y) - cellLevel(layout, b.x, b.y)) !== 1) {
    return { layout, note: `Raise one side of (${a.x}, ${a.y}) and (${b.x}, ${b.y}) by one step first.` };
  }
  const stairs = layout.stairs ?? [];
  const same = stairs.some((stair) => (stair.x === a.x && stair.y === a.y && stair.x2 === b.x && stair.y2 === b.y) || (stair.x === b.x && stair.y === b.y && stair.x2 === a.x && stair.y2 === a.y));
  if (same) return { layout, note: '' };
  const next = cloneLayout(layout);
  next.stairs = [...stairs, { id: crypto.randomUUID(), asset, x: a.x, y: a.y, x2: b.x, y2: b.y }];
  return { layout: next, note: '' };
}

function addLight(layout: MapLayout, x: number, y: number, range: number, color: string, kind: LightKind, intensity: number): MapLayout {
  const cellX = Math.floor(x);
  const cellY = Math.floor(y);
  if (cellX < 0 || cellY < 0 || cellX >= layout.width || cellY >= layout.height) return layout;
  const next = cloneLayout(layout);
  const lights = next.lights ?? [];
  const light = { x: cellX + 0.5, y: cellY + 0.5, range, color, kind, intensity };
  const index = lights.findIndex((item) => Math.floor(item.x) === cellX && Math.floor(item.y) === cellY);
  if (index >= 0) lights[index] = light;
  else lights.push(light);
  next.lights = lights;
  return next;
}

function lightCell(x: number, y: number, width: number, height: number): Cell[] {
  const cell = { x: Math.floor(x), y: Math.floor(y) };
  if (cell.x < 0 || cell.y < 0 || cell.x >= width || cell.y >= height) return [];
  return [cell];
}

function sameEnvironment(a: EnvironmentLighting, b: EnvironmentLighting) {
  return a.preset === b.preset && a.brightness === b.brightness;
}

function normalize(draft: Draft): Draft {
  draft.layout.lights ??= [];
  return draft;
}

function entriesFor(packs: Pack[], themeId: string): ScatterEntry[] {
  const theme = findIn(packs, themeId, (pack) => pack.themes);
  if (!theme) return [];
  return theme.def.scatter.flatMap((entry): ScatterEntry[] => {
    const object = findIn(packs, entry.asset, (pack) => pack.objects ?? []);
    if (object && object.def.kind === 'static') {
      return [{ asset: entry.asset, density: entry.density, spacing: entry.spacing, kind: 'prop' as const, footprint: object.def.footprintCells, rotate: false }];
    }
    const tile = findIn(packs, entry.asset, (pack) => pack.tiles ?? []);
    if (tile?.def.kind === 'overlay') {
      return [{ asset: entry.asset, density: entry.density, spacing: entry.spacing, kind: 'decal' as const, footprint: [1, 1], rotate: true }];
    }
    const prop = findIn(packs, entry.asset, (pack) => pack.props);
    if (prop) {
      return [{ asset: entry.asset, density: entry.density, spacing: entry.spacing, kind: 'prop' as const, footprint: prop.def.footprint, rotate: false }];
    }
    const decal = findIn(packs, entry.asset, (pack) => pack.decals);
    if (!decal) return [];
    return [{ asset: entry.asset, density: entry.density, spacing: entry.spacing, kind: 'decal' as const, footprint: decal.def.size, rotate: !!decal.def.rotate }];
  });
}

function footprintOf(packs: Pack[], asset: string): [number, number] {
  const object = findIn(packs, asset, (pack) => pack.objects ?? []);
  if (object) return [object.def.footprintCells[0], object.def.footprintCells[1]];
  return findIn(packs, asset, (pack) => pack.props)?.def.footprint ?? findIn(packs, asset, (pack) => pack.decals)?.def.size ?? [1, 1];
}

function sizeOf(packs: Pack[], asset: string) {
  const [w, d] = footprintOf(packs, asset);
  return { w, d };
}

export const editor = new EditorStore();
