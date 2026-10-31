import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { combatUrl, playCombat, safeVolume, setCombatVolume, stopCombat } from './combat';

const created: FakeAudio[] = [];
class FakeAudio {
  volume = 1;
  currentTime = 3;
  onended: (() => void) | null = null;
  onerror: (() => void) | null = null;
  pause = vi.fn();
  play = vi.fn(async () => {});
  constructor(readonly src: string) { created.push(this); }
}
beforeEach(() => { created.length = 0; vi.stubGlobal('Audio', FakeAudio); });
afterEach(() => { stopCombat(); vi.unstubAllGlobals(); });

it('keeps both voice selections explicit and never starts muted or zero-volume audio', async () => {
  expect(combatUrl('female', 'hurt')).toBe('/audio/combat/female/hurt.ogg');
  expect(combatUrl('male', 'defeat')).toBe('/audio/combat/male/defeat.ogg');
  expect(await playCombat('female', 'attack', { volume: 1, muted: true })).toBe(false);
  expect(await playCombat('male', 'attack', { volume: 0, muted: false })).toBe(false);
  expect(created).toHaveLength(0);
});

it('replaces the previous cue, updates gain, and cleans up on completion', async () => {
  await playCombat('female', 'attack', { volume: 0.65, muted: false });
  const first = created[0]!;
  const ended = vi.fn();
  await playCombat('male', 'exertion', { volume: 0.4, muted: false, onEnded: ended });
  expect(first.pause).toHaveBeenCalledOnce();
  expect(first.currentTime).toBe(0);
  expect(first.onended).toBeNull();
  setCombatVolume(0.25);
  expect(created[1]!.volume).toBe(0.25);
  created[1]!.onended?.();
  expect(ended).toHaveBeenCalledOnce();
});

it('clamps persisted gain to a safe audio volume', () => {
  expect(safeVolume(8)).toBe(1);
  expect(safeVolume(-1)).toBe(0);
  expect(safeVolume(NaN)).toBe(0.65);
});
