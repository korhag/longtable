// ------------------------------------------------------------
// Module: playback
// Purpose: Pick the frame a die should show for a server time.
//          The numbers and the motion both come from the server.
// ------------------------------------------------------------

export interface Frame {
  p: [number, number, number];
  q: [number, number, number, number];
}

export interface Sample {
  index: number;
  next: number;
  frac: number;
  done: boolean;
}

/** Where `time` (seconds from the first frame) falls in a keyframe list. */
export function sampleFrame(time: number, dt: number, count: number): Sample {
  if (count <= 0) return { index: 0, next: 0, frac: 0, done: true };
  if (count === 1 || time <= 0) return { index: 0, next: 0, frac: 0, done: false };
  const max = (count - 1) * dt;
  if (time >= max) return { index: count - 1, next: count - 1, frac: 0, done: true };
  const scaled = time / dt;
  const index = Math.floor(scaled);
  return { index, next: index + 1, frac: scaled - index, done: false };
}

export function lerpFrame(a: Frame, b: Frame, frac: number): { p: [number, number, number] } {
  return {
    p: [
      a.p[0] + (b.p[0] - a.p[0]) * frac,
      a.p[1] + (b.p[1] - a.p[1]) * frac,
      a.p[2] + (b.p[2] - a.p[2]) * frac,
    ],
  };
}

/** Seconds from the first keyframe to the last. */
export function rollDuration(frameCount: number, dt: number): number {
  if (frameCount <= 1) return 0;
  return (frameCount - 1) * dt;
}

const SETTLED_POSITION = 0.015;
const SETTLED_ROTATION = 0.02;

/** Seconds until the die has stopped. Later frames that barely move do not count. */
export function settleTime(frames: Frame[], dt: number): number {
  if (frames.length <= 1) return 0;
  let lastMove = 0;
  for (let index = 1; index < frames.length; index += 1) {
    const previous = frames[index - 1];
    const current = frames[index];
    if (!previous || !current) continue;
    const moved = Math.hypot(
      current.p[0] - previous.p[0],
      current.p[1] - previous.p[1],
      current.p[2] - previous.p[2],
    );
    const turned = Math.hypot(
      current.q[0] - previous.q[0],
      current.q[1] - previous.q[1],
      current.q[2] - previous.q[2],
      current.q[3] - previous.q[3],
    );
    if (moved > SETTLED_POSITION || turned > SETTLED_ROTATION) lastMove = index;
  }
  return lastMove * dt;
}
