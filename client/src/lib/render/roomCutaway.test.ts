import { describe, expect, it } from 'vitest';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { roomCutaway } from './roomCutaway';

function cabin(): MapLayout {
  const pieces: MapLayout['pieces'] = [];
  for (let n = 2; n < 5; n += 1) {
    pieces.push({ id: `north-${n}`, kind: 'wall', asset: 'forest/wall', axis: 'h', x: n, y: 2, open: false });
    pieces.push({ id: `south-${n}`, kind: n === 3 ? 'door' : 'wall', asset: 'forest/wall', axis: 'h', x: n, y: 5, open: n === 3 });
    pieces.push({ id: `west-${n}`, kind: 'wall', asset: 'forest/wall', axis: 'v', x: 2, y: n, open: false });
    pieces.push({ id: `east-${n}`, kind: 'wall', asset: 'forest/wall', axis: 'v', x: 5, y: n, open: false });
  }
  return { width: 8, height: 8, terrains: [], cells: '', pieces, props: [], decals: [], lights: [], seed: 1 };
}

describe('occupied room cutaways', () => {
  it('lowers foreground walls around an inside occupant, while keeping doors and far walls', () => {
    expect([...roomCutaway(cabin(), [{ x: 3.5, y: 3.5 }])].sort()).toEqual([
      'south-2', 'south-4', 'west-2', 'west-3', 'west-4',
    ]);
  });
  it('keeps all walls full height when everyone is outdoors or no authorized occupant exists', () => {
    expect(roomCutaway(cabin(), [{ x: 1.5, y: 3.5 }]).size).toBe(0);
    expect(roomCutaway(cabin(), []).size).toBe(0);
  });
  it('does not mistake an incomplete wall enclosure for a room', () => {
    const map = cabin();
    map.pieces = map.pieces.filter((piece) => piece.id !== 'north-3');
    expect(roomCutaway(map, [{ x: 3.5, y: 3.5 }]).size).toBe(0);
  });
  it('only cuts the occupied room when two cabins are present', () => {
    const map = cabin();
    map.width = 16;
    map.pieces.push(...map.pieces.map((piece) => ({ ...piece, id: `other-${piece.id}`, x: piece.x + 8 })));
    const cut = [...roomCutaway(map, [{ x: 3.5, y: 3.5 }])];
    expect(cut).toHaveLength(5);
    expect(cut.some((id) => id.startsWith('other-'))).toBe(false);
  });
});
