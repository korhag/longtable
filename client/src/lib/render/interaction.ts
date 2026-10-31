/** Selection is independent of seat role; ownership decides whether control can change. */
export function figurePress(selected: string | null, target: string | null, clicked: string, controllable: boolean): 'select' | 'target' | 'drag' | 'none' {
  if (selected === clicked) return controllable ? 'drag' : 'none';
  if (controllable && (!selected || target === clicked)) return 'select';
  return 'target';
}

export function isClick(start: { x: number; y: number }, end: { clientX: number; clientY: number }): boolean {
  return Math.hypot(end.clientX - start.x, end.clientY - start.y) < 5;
}

/** Feet/cells are unambiguous even when a foreground sprite covers that cell. */
export function figureCandidates(cellIds: string[], opaqueIds: string[]): string[] {
  return [...new Set([...cellIds, ...opaqueIds])];
}
