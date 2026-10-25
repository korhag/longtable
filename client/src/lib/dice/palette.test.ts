import { describe, expect, it } from 'vitest';

import { dieLook, STANDARD_SIDES, type TagColor } from './palette';

const tags: TagColor[] = [
  { letter: 'h', body: '#d4a656', ink: '#1c140e' },
  { letter: 'f', body: '#3c2158', ink: '#f4ecdf' },
];

describe('die colors', () => {
  it('paints tagged dice from the system and leaves the others alone', () => {
    expect(dieLook(12, 'h', tags).body).toBe('#d4a656');
    expect(dieLook(12, 'f', tags).body).toBe('#3c2158');
    for (const sides of STANDARD_SIDES) {
      expect(dieLook(sides, null, tags).body).not.toBe('#d4a656');
      expect(dieLook(sides, null, tags).body).not.toBe('#3c2158');
    }
  });
});
