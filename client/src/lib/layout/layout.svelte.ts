// ------------------------------------------------------------
// Module: layout
// Purpose: Where each panel sits. Docks remember their width.
//          A floating panel remembers its box. Reset clears it.
// ------------------------------------------------------------

const STORAGE_KEY = 'game-table.layout';

export interface PanelPlace {
  floating: boolean;
  collapsed: boolean;
  x: number;
  y: number;
  w: number;
  h: number;
}

interface SavedLayout {
  leftWidth?: number;
  rightWidth?: number;
  panels?: Record<string, PanelPlace>;
  leftOpen?: boolean;
  rightOpen?: boolean;
}

interface FloatingPanel { id: string; closable: boolean }

function read(): SavedLayout {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as SavedLayout;
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

function clamp(value: number | undefined, min: number, max: number, fallback: number) {
  if (typeof value !== 'number' || Number.isNaN(value)) return fallback;
  return Math.min(max, Math.max(min, value));
}

const saved = read();

const EMPTY: PanelPlace = { floating: false, collapsed: false, x: 96, y: 72, w: 380, h: 520 };

class LayoutStore {
  leftWidth = $state(clamp(saved.leftWidth, 240, 520, 320));
  rightWidth = $state(clamp(saved.rightWidth, 260, 520, 340));
  leftOpen = $state(saved.leftOpen ?? false);
  rightOpen = $state(saved.rightOpen ?? false);
  focusMode = $state(false);
  panels = $state<Record<string, PanelPlace>>({ ...(saved.panels ?? {}) });
  // Only mounted windows belong to the stack. Z order is session UI state,
  // so hiding a dock or changing modes cannot leave an invisible top window.
  floatingPanels = $state<FloatingPanel[]>([]);

  registerWindow(id: string, floating: boolean, closable: boolean) {
    if (!floating) { this.removeWindow(id); return; }
    const existing = this.floatingPanels.find(panel => panel.id === id);
    if (existing?.closable === closable) return;
    if (existing) this.floatingPanels = this.floatingPanels.map(panel => panel.id === id ? { id, closable } : panel);
    else this.floatingPanels = [...this.floatingPanels, { id, closable }];
  }

  removeWindow(id: string) {
    if (this.floatingPanels.some(panel => panel.id === id)) this.floatingPanels = this.floatingPanels.filter(panel => panel.id !== id);
  }

  raiseWindow(id: string) {
    const panel = this.floatingPanels.find(panel => panel.id === id);
    if (!panel || this.frontmostWindow === id) return;
    this.floatingPanels = [...this.floatingPanels.filter(panel => panel.id !== id), panel];
  }

  windowDepth(id: string) { return Math.max(0, this.floatingPanels.findIndex(panel => panel.id === id)); }
  get frontmostWindow() { return this.floatingPanels.at(-1)?.id ?? null; }
  get frontmostClosableWindow() {
    for (let index = this.floatingPanels.length - 1; index >= 0; index--) {
      const panel = this.floatingPanels[index];
      if (panel?.closable) return panel.id;
    }
    return null;
  }

  place(id: string): PanelPlace {
    return this.panels[id] ?? EMPTY;
  }

  patch(id: string, next: Partial<PanelPlace>) {
    this.panels = { ...this.panels, [id]: { ...this.place(id), ...next } };
    this.save();
  }

  setDock(side: 'left' | 'right', width: number) {
    const next = clamp(width, side === 'left' ? 240 : 260, 520, side === 'left' ? 320 : 340);
    if (side === 'left') this.leftWidth = next;
    else this.rightWidth = next;
    this.save();
  }

  toggleDock(side: 'left' | 'right') {
    this.focusMode = false;
    if (side === 'left') this.leftOpen = !this.leftOpen;
    else this.rightOpen = !this.rightOpen;
    if (window.innerWidth < 1000) {
      if (side === 'left' && this.leftOpen) this.rightOpen = false;
      if (side === 'right' && this.rightOpen) this.leftOpen = false;
    }
    this.save();
  }

  reset() {
    try { localStorage.removeItem(STORAGE_KEY); } catch { /* Storage may be disabled. */ }
    this.leftWidth = 320;
    this.rightWidth = 340;
    this.panels = {};
    this.leftOpen = false;
    this.rightOpen = false;
    this.focusMode = false;
  }

  private save() {
    const payload: SavedLayout = {
      leftWidth: this.leftWidth,
      rightWidth: this.rightWidth,
      panels: this.panels,
      leftOpen: this.leftOpen,
      rightOpen: this.rightOpen,
    };
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(payload)); } catch { /* Private browsing can disable storage. */ }
  }
}

export const layout = new LayoutStore();
