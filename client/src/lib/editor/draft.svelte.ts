// ------------------------------------------------------------
// Module: draft
// Purpose: The GM's map draft. Strokes are undone as a whole,
//          and the draft is saved shortly after the last change.
// ------------------------------------------------------------

import { findIn, loadCatalog, type Pack } from '../assets/catalog';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { PlacedDecal } from '../net/bindings/PlacedDecal.ts';
import type { PlacedProp } from '../net/bindings/PlacedProp.ts';
import { table } from '../table.svelte';
import { brushCells, nearestEdge, type BrushSize, type Cell } from './brush';
import { decodeCells, encodeCells } from './cells';
import { blocksDoor, fillTheme, footprintsOverlap, type ScatterEntry } from './scatter';

export type BuildTool = 'theme' | 'terrain' | 'wall' | 'door' | 'prop' | 'decal' | 'erase';

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
  layout: MapLayout;
}

class EditorStore {
  active = $state(false);
  tool = $state<BuildTool>('theme');
  brush = $state<BrushSize>(3);
  themeId = $state('core/grassland');
  terrainId = $state('core/grass');
  wallId = $state('core/hedge');
  doorId = $state('core/hedge-gate');
  propId = $state('core/oak');
  decalId = $state('core/tuft');
  mirror = $state(false);
  packs = $state<Pack[]>([]);
  summaries = $state<DraftSummary[]>([]);
  draft = $state<Draft | null>(null);
  ghost = $state<Cell[]>([]);
  note = $state('');
  private past: MapLayout[] = [];
  private future: MapLayout[] = [];
  private timer = 0;
  private stroke: Cell[] = [];
  private base: MapLayout | null = null;
  private painting = false;

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
    this.draft = (await response.json()) as Draft;
    this.past = [];
    this.future = [];
    await this.refresh();
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
    this.draft = (await response.json()) as Draft;
    await this.refresh();
  }

  async load(id: string) {
    const response = await fetch(`/api/drafts/${id}`, { headers: this.headers() });
    if (!response.ok) return;
    this.draft = (await response.json()) as Draft;
    this.past = [];
    this.future = [];
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
    await this.save();
    const response = await fetch(`/api/drafts/${this.draft.id}/publish`, { method: 'POST', headers: this.headers() });
    this.note = response.ok ? 'On the table' : await response.text();
  }

  undo() {
    if (!this.draft || this.past.length === 0 || this.painting) return;
    this.future.push(structuredClone(this.draft.layout));
    this.draft.layout = this.past.pop() ?? this.draft.layout;
    this.schedule();
  }

  redo() {
    if (!this.draft || this.future.length === 0 || this.painting) return;
    this.past.push(structuredClone(this.draft.layout));
    this.draft.layout = this.future.pop() ?? this.draft.layout;
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
      this.ghost = this.paints() ? brushCells(point.x, point.y, this.brush, current.width, current.height) : [];
      return;
    }
    if (button === 2) {
      if (phase === 'down' && this.tool === 'prop') this.mirror = !this.mirror;
      return;
    }
    if (button !== 0) return;
    if (phase === 'down') {
      this.base = structuredClone(current);
      this.stroke = [];
      this.painting = true;
      this.step(point);
    } else if (phase === 'move' && this.painting) {
      this.step(point);
    } else if (phase === 'up' && this.painting) {
      this.finish();
      this.painting = false;
      this.base = null;
      this.stroke = [];
    }
  }

  private paints() {
    return this.tool === 'theme' || this.tool === 'terrain' || this.tool === 'erase';
  }

  private step(point: { x: number; y: number }) {
    const layout = this.draft?.layout;
    const base = this.base;
    if (!layout || !base || !this.draft) return;
    if (this.paints()) {
      for (const cell of brushCells(point.x, point.y, this.brush, layout.width, layout.height)) {
        if (!this.stroke.some((item) => item.x === cell.x && item.y === cell.y)) this.stroke.push(cell);
      }
      this.ghost = this.stroke;
      if (this.tool === 'terrain') this.draft.layout = paintTerrain(base, this.stroke, this.terrainId);
      return;
    }
    if (this.tool === 'wall') {
      const edge = nearestEdge(point.x, point.y, layout.width, layout.height);
      if (edge) this.draft.layout = addPiece(this.draft.layout, edge, 'wall', this.wallId);
      return;
    }
    if (this.stroke.length > 0) return;
    this.stroke.push({ x: Math.floor(point.x), y: Math.floor(point.y) });
    if (this.tool === 'door') {
      const edge = nearestEdge(point.x, point.y, layout.width, layout.height);
      if (edge) this.draft.layout = addPiece(layout, edge, 'door', this.doorId);
    } else if (this.tool === 'prop') {
      this.draft.layout = addProp(layout, Math.floor(point.x), Math.floor(point.y), this.propId, this.mirror, this.packs);
    } else if (this.tool === 'decal') {
      this.draft.layout = addDecal(layout, point.x, point.y, this.decalId, this.packs);
    }
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

  private async save() {
    if (!this.draft) return;
    await fetch(`/api/drafts/${this.draft.id}`, {
      method: 'PUT',
      headers: this.headers(),
      body: JSON.stringify(this.draft),
    });
  }

  private headers(): Record<string, string> {
    return {
      Authorization: `Bearer ${table.sessionToken ?? ''}`,
      'Content-Type': 'application/json',
    };
  }
}

function paintTerrain(base: MapLayout, cells: Cell[], terrain: string): MapLayout {
  const next = structuredClone(base);
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
  return next;
}

function addPiece(layout: MapLayout, edge: { x: number; y: number; axis: 'h' | 'v' }, kind: 'wall' | 'door', asset: string): MapLayout {
  const next = structuredClone(layout);
  const index = next.pieces.findIndex((piece) => piece.x === edge.x && piece.y === edge.y && piece.axis === edge.axis);
  if (index >= 0) {
    const existing = next.pieces[index];
    if (!existing || existing.kind === kind) return layout;
    next.pieces[index] = { ...existing, kind, asset, open: false };
    return next;
  }
  next.pieces.push({ id: crypto.randomUUID(), kind, asset, x: edge.x, y: edge.y, axis: edge.axis, open: false });
  return next;
}

function addProp(layout: MapLayout, x: number, y: number, asset: string, mirror: boolean, packs: Pack[]): MapLayout {
  const [w, d] = footprintOf(packs, asset);
  const foot = { x, y, w, d };
  if (x < 0 || y < 0 || x + w > layout.width || y + d > layout.height) return layout;
  if (blocksDoor(foot, layout)) return layout;
  const busy = layout.props.some((prop) => footprintsOverlap(foot, { x: prop.x, y: prop.y, ...sizeOf(packs, prop.asset) }));
  if (busy) return layout;
  const placed: PlacedProp = { id: crypto.randomUUID(), asset, x, y, mirror };
  return { ...layout, props: [...layout.props, placed] };
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
  const next = structuredClone(base);
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
  return next;
}

function entriesFor(packs: Pack[], themeId: string): ScatterEntry[] {
  const theme = findIn(packs, themeId, (pack) => pack.themes);
  if (!theme) return [];
  return theme.def.scatter.flatMap((entry): ScatterEntry[] => {
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
  return findIn(packs, asset, (pack) => pack.props)?.def.footprint ?? findIn(packs, asset, (pack) => pack.decals)?.def.size ?? [1, 1];
}

function sizeOf(packs: Pack[], asset: string) {
  const [w, d] = footprintOf(packs, asset);
  return { w, d };
}

export const editor = new EditorStore();
