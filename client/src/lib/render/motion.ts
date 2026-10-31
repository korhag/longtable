import type { WalkPoint } from './route';
export interface WalkMotion { points: WalkPoint[]; lengths: number[]; distance: number; started: number; duration: number }
export function walkMotion(points: WalkPoint[], started: number): WalkMotion {
  const lengths = [0];
  for (let i = 1; i < points.length; i += 1) {
    const a = points[i - 1]!, b = points[i]!;
    lengths.push(lengths[i - 1]! + Math.hypot(b.x - a.x, b.z - a.z));
  }
  const distance = lengths.at(-1) ?? 0;
  return { points, lengths, distance, started, duration: Math.max(100, Math.min(8000, distance / 6 * 1000)) };
}
export function motionPosition(motion: WalkMotion, now: number): { x: number; y: number; z: number; done: boolean } {
  const progress = Math.min(1, Math.max(0, (now - motion.started) / motion.duration));
  const travelled = progress * motion.distance;
  for (let i = 1; i < motion.points.length; i += 1) {
    if (travelled > motion.lengths[i]!) continue;
    const a = motion.points[i - 1]!, b = motion.points[i]!;
    const span = motion.lengths[i]! - motion.lengths[i - 1]!;
    const t = span ? (travelled - motion.lengths[i - 1]!) / span : 1;
    return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, z: a.z + (b.z - a.z) * t, done: progress >= 1 };
  }
  return { ...(motion.points.at(-1) ?? { x: 0, y: 0, z: 0 }), done: true };
}

export function motionFacing(motion: WalkMotion, now: number, fallback: string): string {
  const a = motionPosition(motion, now);
  const b = motionPosition(motion, now + 20);
  const dx = b.x - a.x, dy = b.z - a.z;
  if (Math.abs(dx) + Math.abs(dy) < 0.001) return fallback;
  return Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'east' : 'west') : (dy > 0 ? 'south' : 'north');
}
