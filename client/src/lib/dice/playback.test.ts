import { describe, expect, it } from 'vitest';

import { lerpFrame, rollDuration, sampleFrame, settleTime, type Frame } from './playback';

describe('dice playback', () => {
  it('waits on the first frame, then lands on the last', () => {
    expect(sampleFrame(-0.1, 1 / 30, 31).index).toBe(0);
    const mid = sampleFrame(0.5, 1 / 30, 31);
    expect(mid.done).toBe(false);
    expect(mid.index).toBe(15);
    expect(sampleFrame(5, 1 / 30, 31).done).toBe(true);
    expect(rollDuration(31, 1 / 30)).toBeCloseTo(1);
  });

  it('settles when the die stops, not at the last frame', () => {
    const still: Frame = { p: [1, 0, 0], q: [0, 0, 0, 1] };
    const frames: Frame[] = [
      { p: [0, 0, 0], q: [0, 0, 0, 1] },
      still,
      still,
      { p: [1.001, 0, 0], q: [0, 0, 0, 1] },
    ];
    expect(settleTime(frames, 0.1)).toBeCloseTo(0.1);
    expect(settleTime(frames, 0.1)).toBeLessThan(rollDuration(frames.length, 0.1));
  });

  it('blends positions between frames', () => {
    const blended = lerpFrame(
      { p: [0, 0, 0], q: [0, 0, 0, 1] },
      { p: [2, 4, 6], q: [0, 0, 0, 1] },
      0.25,
    );
    expect(blended.p).toEqual([0.5, 1, 1.5]);
  });
});
