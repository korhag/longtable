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
