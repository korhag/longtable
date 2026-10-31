import { expect, it } from 'vitest';
import { figurePress, isClick } from './interaction';

it('selects first owned figure, targets another, then selects that target on repeat click', () => {
  expect(figurePress(null, null, 'elf', true)).toBe('select');
  expect(figurePress('elf', null, 'dwarf', true)).toBe('target');
  expect(figurePress('elf', 'dwarf', 'dwarf', true)).toBe('select');
  expect(figurePress('dwarf', null, 'dwarf', true)).toBe('drag');
});
it('never takes control of someone else’s figure, including a repeated target click', () => {
  expect(figurePress(null, null, 'bandit', false)).toBe('target');
  expect(figurePress('elf', 'bandit', 'bandit', false)).toBe('target');
});
it('separates a destination click from camera dragging', () => {
  expect(isClick({ x: 100, y: 50 }, { clientX: 102, clientY: 51 })).toBe(true);
  expect(isClick({ x: 100, y: 50 }, { clientX: 110, clientY: 50 })).toBe(false);
});
