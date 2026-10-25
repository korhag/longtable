// ------------------------------------------------------------
// Module: palette
// Purpose: Colors for the six dice. A system's die tags, such
//          as Hope and Fear, paint over those when a roll uses them.
// ------------------------------------------------------------

export interface DieLook {
  body: string;
  ink: string;
  stroke: string;
  bodyHex: number;
}

export interface TagColor {
  letter: string;
  body: string;
  ink: string;
}

const BY_SIDES: Record<number, DieLook> = {
  4: { body: '#c45c4a', ink: '#f6efe4', stroke: '#2a120e', bodyHex: 0xc45c4a },
  6: { body: '#f3eee4', ink: '#1c140e', stroke: '#f6efe4', bodyHex: 0xf3eee4 },
  8: { body: '#6e9a72', ink: '#f4ecdf', stroke: '#142018', bodyHex: 0x6e9a72 },
  10: { body: '#2a8f84', ink: '#f4ecdf', stroke: '#102422', bodyHex: 0x2a8f84 },
  12: { body: '#6f8fbf', ink: '#f4ecdf', stroke: '#101820', bodyHex: 0x6f8fbf },
  20: { body: '#262320', ink: '#f6efe4', stroke: '#f2d48a', bodyHex: 0x262320 },
};

export function dieLook(sides: number, tag?: string | null, tags: TagColor[] = []): DieLook {
  const painted = tag ? tags.find((item) => item.letter === tag) : undefined;
  if (painted) {
    return { body: painted.body, ink: painted.ink, stroke: '#f6efe4', bodyHex: hexColor(painted.body) };
  }
  return BY_SIDES[sides] ?? BY_SIDES[6]!;
}

function hexColor(body: string): number {
  const value = Number.parseInt(body.replace('#', ''), 16);
  return Number.isFinite(value) ? value : 0xf3eee4;
}

export const STANDARD_SIDES = [4, 6, 8, 10, 12, 20] as const;
