/** Selection is independent of seat role; ownership decides whether control can change. */
export function figurePress(selected: string | null, target: string | null, clicked: string, controllable: boolean): 'select' | 'target' | 'drag' | 'none' {
  if (selected === clicked) return controllable ? 'drag' : 'none';
  if (controllable && (!selected || target === clicked)) return 'select';
  return 'target';
}

export function isClick(start: { x: number; y: number }, end: { clientX: number; clientY: number }): boolean {
  return Math.hypot(end.clientX - start.x, end.clientY - start.y) < 5;
}
