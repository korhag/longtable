// ------------------------------------------------------------
// Module: sound
// Purpose: Small synthesized dice sounds. No audio files, so
//          the table stays quiet to download and easy to host.
// ------------------------------------------------------------

let context: AudioContext | null = null;

function audio(): AudioContext | null {
  const Ctx = window.AudioContext;
  if (!Ctx) return null;
  context ??= new Ctx();
  if (context.state === 'suspended') void context.resume();
  return context;
}

export function playRollStart() {
  const ctx = audio();
  if (!ctx) return;
  const noise = noiseBurst(ctx, 0.18, 0.05);
  const thump = ctx.createOscillator();
  const gain = ctx.createGain();
  thump.type = 'sine';
  thump.frequency.setValueAtTime(140, ctx.currentTime);
  thump.frequency.exponentialRampToValueAtTime(55, ctx.currentTime + 0.16);
  gain.gain.setValueAtTime(0.0001, ctx.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.12, ctx.currentTime + 0.02);
  gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.2);
  thump.connect(gain);
  gain.connect(ctx.destination);
  thump.start();
  thump.stop(ctx.currentTime + 0.22);
  noise.connect(ctx.destination);
}

export interface Impact {
  time: number;
  strength: number;
}

/** Bounces: the die was falling and then rises, or it suddenly slows near the tray. */
export function impacts(frames: { p: [number, number, number] }[], dt: number): Impact[] {
  const hits: Impact[] = [];
  if (frames.length < 3 || dt <= 0) return hits;
  for (let index = 1; index < frames.length - 1; index += 1) {
    const previous = frames[index - 1]?.p;
    const current = frames[index]?.p;
    const next = frames[index + 1]?.p;
    if (!previous || !current || !next) continue;
    const fall = current[1] - previous[1];
    const rise = next[1] - current[1];
    const before = Math.hypot(current[0] - previous[0], current[1] - previous[1], current[2] - previous[2]);
    const after = Math.hypot(next[0] - current[0], next[1] - current[1], next[2] - current[2]);
    const bounced = fall < -0.002 && rise >= -0.001;
    const slowed = before > 0.02 && after < before * 0.55 && current[1] < 0.35;
    if (bounced || slowed) hits.push({ time: index * dt, strength: Math.min(1, before * 4) });
  }
  return hits;
}

const TIMBRE: Record<number, { frequency: number; decay: number; kind: OscillatorType }> = {
  4: { frequency: 1800, decay: 0.04, kind: 'square' },
  6: { frequency: 420, decay: 0.07, kind: 'triangle' },
  8: { frequency: 680, decay: 0.06, kind: 'triangle' },
  10: { frequency: 520, decay: 0.07, kind: 'sine' },
  12: { frequency: 360, decay: 0.08, kind: 'sine' },
  20: { frequency: 180, decay: 0.12, kind: 'sine' },
};

const PITCH = [0.9, 1, 1.12];

export function playBounces(
  sides: number,
  hits: Impact[],
  playAt: number,
  serverNow: () => number,
  x: number,
) {
  if (localStorage.getItem('game-table.sound') === 'off') return;
  const ctx = audio();
  if (!ctx) return;
  const elapsed = (serverNow() - playAt) / 1000;
  const voice = TIMBRE[sides] ?? TIMBRE[6]!;
  const pitch = PITCH[Math.floor(Math.random() * PITCH.length)] ?? 1;
  for (const hit of hits.slice(0, 8)) {
    if (hit.time < elapsed - 0.05) continue;
    click(ctx, ctx.currentTime + Math.max(0, hit.time - elapsed), voice, pitch, hit.strength, x);
  }
}

function click(
  ctx: AudioContext,
  when: number,
  voice: { frequency: number; decay: number; kind: OscillatorType },
  pitch: number,
  strength: number,
  x: number,
) {
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  const pan = ctx.createStereoPanner();
  osc.type = voice.kind;
  osc.frequency.setValueAtTime(voice.frequency * pitch, when);
  gain.gain.setValueAtTime(0.0001, when);
  gain.gain.exponentialRampToValueAtTime(0.08 * (0.35 + strength), when + 0.008);
  gain.gain.exponentialRampToValueAtTime(0.0001, when + voice.decay);
  pan.pan.value = Math.max(-1, Math.min(1, x / 8));
  osc.connect(gain);
  const noise = scheduledNoise(ctx, voice.decay, 0.04 * (0.4 + strength));
  noise.connect(gain);
  gain.connect(pan);
  pan.connect(ctx.destination);
  osc.start(when);
  osc.stop(when + voice.decay + 0.02);
  noise.start(when);
}

function scheduledNoise(ctx: AudioContext, seconds: number, volume: number) {
  const length = Math.max(1, Math.floor(ctx.sampleRate * seconds));
  const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
  const data = buffer.getChannelData(0);
  for (let index = 0; index < length; index += 1) {
    const fade = 1 - index / length;
    data[index] = (Math.random() * 2 - 1) * fade * fade;
  }
  const source = ctx.createBufferSource();
  source.buffer = buffer;
  const gain = ctx.createGain();
  gain.gain.value = volume;
  source.connect(gain);
  return { connect: (node: AudioNode) => gain.connect(node), start: (when: number) => source.start(when) };
}

export function playRollSettle() {
  const ctx = audio();
  if (!ctx) return;
  const noise = noiseBurst(ctx, 0.08, 0.03);
  noise.connect(ctx.destination);
}

function noiseBurst(ctx: AudioContext, seconds: number, volume: number) {
  const length = Math.floor(ctx.sampleRate * seconds);
  const buffer = ctx.createBuffer(1, length, ctx.sampleRate);
  const data = buffer.getChannelData(0);
  for (let index = 0; index < length; index += 1) {
    const fade = 1 - index / length;
    data[index] = (Math.random() * 2 - 1) * fade * fade;
  }
  const source = ctx.createBufferSource();
  source.buffer = buffer;
  const gain = ctx.createGain();
  gain.gain.value = volume;
  source.connect(gain);
  source.start();
  return gain;
}
