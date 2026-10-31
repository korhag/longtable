export const COMBAT_VOICES = ['female', 'male'] as const;
export type CombatVoice = (typeof COMBAT_VOICES)[number];
export const COMBAT_ACTIONS = [
  { id: 'attack', label: 'Attack' },
  { id: 'exertion', label: 'Exertion' },
  { id: 'hurt', label: 'Take damage' },
  { id: 'defeat', label: 'Defeat' },
] as const;
export type CombatAction = (typeof COMBAT_ACTIONS)[number]['id'];

let current: HTMLAudioElement | null = null;
export function combatUrl(voice: CombatVoice, action: CombatAction) {
  return `/audio/combat/${voice}/${action}.ogg`;
}
export function safeVolume(value: number) {
  return Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0.65;
}
export function stopCombat() {
  if (!current) return;
  current.pause();
  current.currentTime = 0;
  current.onended = null;
  current.onerror = null;
  current = null;
}
export function setCombatVolume(value: number) {
  if (current) current.volume = safeVolume(value);
}

/** Called only from a user gesture. A second cue replaces the first, rather than stacking voices. */
export async function playCombat(voice: CombatVoice, action: CombatAction, options: {
  volume: number; muted: boolean; onEnded?: () => void;
}): Promise<boolean> {
  stopCombat();
  const volume = safeVolume(options.volume);
  if (options.muted || volume === 0) return false;
  const sound = new Audio(combatUrl(voice, action));
  current = sound;
  sound.volume = volume;
  const finish = () => {
    if (current !== sound) return;
    current = null;
    options.onEnded?.();
  };
  sound.onended = finish;
  sound.onerror = finish;
  try {
    await sound.play();
    return current === sound;
  } catch (error) {
    if (current === sound) stopCombat();
    throw error;
  }
}
