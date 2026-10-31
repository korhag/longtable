import { beforeEach, describe, expect, it } from 'vitest';
import { layout } from './layout.svelte';

describe('mounted floating windows', () => {
  beforeEach(() => { layout.floatingPanels = []; });

  it('raises an interacted window and restores the next window after it closes', () => {
    layout.registerWindow('sheet', true, true);
    layout.registerWindow('settings', true, true);
    expect(layout.frontmostClosableWindow).toBe('settings');
    layout.raiseWindow('sheet');
    expect(layout.windowDepth('sheet')).toBeGreaterThan(layout.windowDepth('settings'));
    expect(layout.frontmostWindow).toBe('sheet');
    expect(layout.frontmostClosableWindow).toBe('sheet');
    layout.removeWindow('sheet');
    expect(layout.frontmostClosableWindow).toBe('settings');
  });

  it('does not let a docked or unmounted panel capture window dismissal', () => {
    layout.registerWindow('settings', true, true);
    layout.registerWindow('sheet', true, true);
    layout.registerWindow('sheet', false, true);
    layout.raiseWindow('sheet');
    expect(layout.frontmostClosableWindow).toBe('settings');
    layout.removeWindow('settings');
    expect(layout.frontmostWindow).toBeNull();
    expect(layout.frontmostClosableWindow).toBeNull();
  });

  it('keeps a nonclosable dice window above the nearest closable window without duplicate entries', () => {
    layout.registerWindow('sheet', true, true);
    layout.registerWindow('dice', true, false);
    layout.registerWindow('sheet', true, true);
    expect(layout.frontmostWindow).toBe('dice');
    expect(layout.frontmostClosableWindow).toBe('sheet');
    expect(layout.floatingPanels).toHaveLength(2);
  });
});
