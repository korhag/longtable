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
