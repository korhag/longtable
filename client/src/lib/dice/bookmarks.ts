// ------------------------------------------------------------
// Module: bookmarks
// Purpose: Named dice setups kept in this browser.
// ------------------------------------------------------------

export interface DiceBookmark {
  label: string;
  formula: string;
  plus: number;
  minus: number;
}

export interface DiceSetup {
  label: string;
  formula: string;
  plus: number;
  minus: number;
}

const STORAGE_KEY = 'game-table.dice-bookmarks';

export function loadBookmarks(): DiceBookmark[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isBookmark);
  } catch {
    return [];
  }
}

export function saveBookmarks(bookmarks: DiceBookmark[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(bookmarks));
}

export function upsertBookmark(bookmarks: DiceBookmark[], next: DiceBookmark): DiceBookmark[] {
  const label = next.label.trim();
  const kept = bookmarks.filter((item) => item.label.trim().toLowerCase() !== label.toLowerCase());
  return [...kept, { label, formula: next.formula, plus: next.plus, minus: next.minus }];
}

export function removeBookmark(bookmarks: DiceBookmark[], label: string): DiceBookmark[] {
  const key = label.trim().toLowerCase();
  return bookmarks.filter((item) => item.label.trim().toLowerCase() !== key);
}

export function bookmarkMatches(bookmark: DiceBookmark, setup: DiceSetup): boolean {
  return bookmark.label.trim().toLowerCase() === setup.label.trim().toLowerCase()
    && bookmark.formula.trim() === setup.formula.trim()
    && bookmark.plus === setup.plus
    && bookmark.minus === setup.minus;
}

function isBookmark(value: unknown): value is DiceBookmark {
  if (!value || typeof value !== 'object') return false;
  const item = value as Partial<DiceBookmark>;
  return typeof item.label === 'string'
    && typeof item.formula === 'string'
    && typeof item.plus === 'number'
    && typeof item.minus === 'number';
}
