// ------------------------------------------------------------
// Module: prefs
// Purpose: The look and the dice tray for the signed-in account.
//          The hearth theme is the one the table already used.
// ------------------------------------------------------------

export interface AccountSettings {
  theme: string;
  parchment: string;
  gold: string;
  diceTray: string;
  diceCorner: string;
}

export const THEMES: Record<string, { parchment: string; gold: string; ink: string; leather: string; felt: string }> = {
  hearth: { parchment: '#f4ecdf', gold: '#d4a656', ink: '#140f0c', leather: '#2a1c14', felt: '#1b3a2c' },
  ember: { parchment: '#f6e6d4', gold: '#e07a3d', ink: '#140f0c', leather: '#2a120c', felt: '#3a1c14' },
  night: { parchment: '#e4e0d8', gold: '#c9a36a', ink: '#0c0e12', leather: '#141820', felt: '#1a2430' },
  sea: { parchment: '#e7f2ec', gold: '#3f8f78', ink: '#0c1410', leather: '#14241c', felt: '#1b3a32' },
};

export const DEFAULT_SETTINGS: AccountSettings = {
  theme: 'hearth',
  parchment: THEMES.hearth.parchment,
  gold: THEMES.hearth.gold,
  diceTray: 'quarter',
  diceCorner: 'bottom-right',
};

function mix(color: string, toward: number, amount: number): string {
  const hex = color.replace('#', '');
  if (hex.length !== 6) return color;
  const channel = (index: number) => Math.round(Number.parseInt(hex.slice(index, index + 2), 16) * (1 - amount) + toward * amount);
  return `#${[channel(0), channel(2), channel(4)].map((part) => part.toString(16).padStart(2, '0')).join('')}`;
}

export function applyLook(settings: AccountSettings) {
  const theme = THEMES[settings.theme] ?? THEMES.hearth;
  const root = document.documentElement;
  root.style.setProperty('--color-parchment', settings.parchment || theme.parchment);
  root.style.setProperty('--color-gold', settings.gold || theme.gold);
  root.style.setProperty('--color-gold-bright', mix(settings.gold || theme.gold, 255, 0.42));
  root.style.setProperty('--color-ink', theme.ink);
  root.style.setProperty('--color-leather', theme.leather);
  root.style.setProperty('--color-felt', theme.felt);
}

class Prefs {
  theme = $state(DEFAULT_SETTINGS.theme);
  parchment = $state(DEFAULT_SETTINGS.parchment);
  gold = $state(DEFAULT_SETTINGS.gold);
  diceTray = $state(DEFAULT_SETTINGS.diceTray);
  diceCorner = $state(DEFAULT_SETTINGS.diceCorner);

  get current(): AccountSettings {
    return {
      theme: this.theme,
      parchment: this.parchment,
      gold: this.gold,
      diceTray: this.diceTray,
      diceCorner: this.diceCorner,
    };
  }

  adopt(settings: AccountSettings) {
    this.theme = settings.theme || 'hearth';
    this.parchment = settings.parchment || THEMES.hearth.parchment;
    this.gold = settings.gold || THEMES.hearth.gold;
    this.diceTray = settings.diceTray === 'cover' ? 'cover' : 'quarter';
    this.diceCorner = settings.diceCorner || 'bottom-right';
    applyLook(this.current);
  }

  useTheme(theme: string) {
    const next = THEMES[theme] ?? THEMES.hearth;
    this.theme = theme in THEMES ? theme : 'hearth';
    this.parchment = next.parchment;
    this.gold = next.gold;
    applyLook(this.current);
  }
}

export const prefs = new Prefs();
