import { describe, expect, it } from 'vitest';

import { castFor, ringPaint } from './rings';

describe('token rings', () => {
  const players = [{ id: 'ada', color: '#7eb8a2' }];
  const characters = [
    { id: 'pc', kind: 'pc', ownerId: 'ada' },
    { id: 'bandit', kind: 'adversary', ownerId: null },
  ];

  it('paints a player character in that player color', () => {
    expect(castFor({ characterId: 'pc', ownerId: null }, characters, players)).toEqual({ tone: 'player', color: '#7eb8a2' });
  });

  it('lets the GM read white, gray, and red', () => {
    expect(ringPaint('player', '#7eb8a2', null, true)).toMatchObject({ color: '#6e675e', ticks: false });
    expect(ringPaint('other', '#6e675e', 'control', true)).toMatchObject({ color: '#f4f1ea', scale: 1.34, ticks: false });
    expect(ringPaint('other', '#6e675e', 'target', true)).toMatchObject({ color: '#e23b32', ticks: false });
  });

  it('shows the GM cast in gray, and a target with red ticks', () => {
    expect(castFor({ characterId: 'bandit', ownerId: null }, characters, players).tone).toBe('other');
    expect(ringPaint('other', '#6e675e', null, false)).toMatchObject({ color: '#6e675e', ticks: false });
    expect(ringPaint('other', '#6e675e', 'target', false)).toMatchObject({ color: '#e23b32', ticks: true });
  });

  it('gives a targeted player character a second brighter stroke', () => {
    const paint = ringPaint('player', '#7eb8a2', 'target', false);
    expect(paint.ticks).toBe(false);
    expect(paint.halo).toBeTruthy();
    expect(paint.halo).not.toBe('#7eb8a2');
  });
});
