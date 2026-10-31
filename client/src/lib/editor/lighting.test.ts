import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { editor } from './draft.svelte';
import { encodeCells } from './cells';
import { DAYLIGHT, ENVIRONMENT_PRESETS } from '../lightingPresets';

beforeEach(() => {
  vi.useFakeTimers();
  vi.stubGlobal('window', { setTimeout, clearTimeout });
  editor.draft = { id: 'lighting-test', sceneId: 'live', name: 'Lighting', updatedAt: 0, layout: {
    width: 12, height: 12, terrains: ['core/grass'], cells: encodeCells(new Uint8Array(144)), pieces: [], props: [], decals: [], lights: [],
  } };
});
afterEach(() => { vi.clearAllTimers(); vi.useRealTimers(); vi.unstubAllGlobals(); });
const click = () => {
  editor.pointer('down', { x: 2.2, y: 3.2 }, 0);
  editor.pointer('up', { x: 2.2, y: 3.2 }, 0);
};
it('places torch light, replaces it with magic, and undoes the replacement', () => {
  editor.chooseLight('torch'); click();
  expect(editor.draft!.layout.lights).toEqual([{ x: 2.5, y: 3.5, kind: 'torch', color: '#ffad42', range: 5, intensity: 1.4 }]);
  editor.chooseLight('magic'); click();
  expect(editor.draft!.layout.lights).toHaveLength(1);
  expect(editor.draft!.layout.lights[0]).toMatchObject({ kind: 'magic', color: '#79c8ff', intensity: 1.3 });
  editor.undo();
  expect(editor.draft!.layout.lights[0]?.kind).toBe('torch');
});
it('previews brightness without creating a separate undo entry for every slider position', () => {
  expect(editor.draft!.layout.environment ?? DAYLIGHT).toEqual({ preset: 'day', brightness: 1 });
  editor.setEnvironment({ preset: 'night', brightness: 0.3 }, false);
  editor.setEnvironment({ preset: 'night', brightness: 0.2 }, false);
  editor.setEnvironment({ preset: 'night', brightness: 0.16 }, true);
  expect(editor.draft!.layout.environment).toEqual(ENVIRONMENT_PRESETS[2]?.value);
  editor.undo();
  expect(editor.draft!.layout.environment ?? DAYLIGHT).toEqual(DAYLIGHT);
  editor.redo();
  expect(editor.draft!.layout.environment?.preset).toBe('night');
});
it('keeps a live time-of-day change when an unrelated draft change is undone', () => {
  editor.chooseLight('torch'); click();
  editor.syncLiveEnvironment('unrelated', { preset: 'night', brightness: 0.16 });
  expect(editor.draft!.layout.environment).toBeUndefined();
  editor.syncLiveEnvironment('live', { preset: 'dusk', brightness: 0.55 });
  editor.undo();
  expect(editor.draft!.layout.lights).toHaveLength(0);
  expect(editor.draft!.layout.environment).toEqual({ preset: 'dusk', brightness: 0.55 });
});

it('publishes unrelated draft edits with the live environment rather than stale draft lighting', async () => {
  const { table } = await import('../table.svelte');
  const previous = table.scene;
  const initial = JSON.parse(JSON.stringify(editor.draft));
  let submitted: typeof initial;
  vi.stubGlobal('fetch', vi.fn(async (_url: string, options?: RequestInit) => {
    if (options?.method === 'PUT') submitted = JSON.parse(String(options.body));
    return { ok: true, json: async () => initial };
  }));
  await editor.load('lighting-test');
  table.scene = { id: 'live', environment: { preset: 'night', brightness: 0.16 } } as NonNullable<typeof table.scene>;
  try {
    editor.chooseLight('torch'); click();
    await editor.publish();
    expect(submitted.layout.environment).toEqual({ preset: 'night', brightness: 0.16 });
    expect(submitted.layout.lights).toHaveLength(1);
  } finally { table.scene = previous; }
});

it('accepts a later live environment after publishing an earlier draft lighting change', async () => {
  const { table } = await import('../table.svelte');
  const previous = table.scene;
  let saved = JSON.parse(JSON.stringify(editor.draft));
  vi.stubGlobal('fetch', vi.fn(async (_url: string, options?: RequestInit) => {
    if (options?.method === 'PUT') saved = JSON.parse(String(options.body));
    return { ok: true, json: async () => JSON.parse(JSON.stringify(saved)) };
  }));
  await editor.load('lighting-test');
  table.scene = { id: 'live', environment: { ...DAYLIGHT } } as NonNullable<typeof table.scene>;
  try {
    editor.setEnvironment({ preset: 'night', brightness: 0.16 });
    await editor.publish();
    expect(saved.layout.environment.preset).toBe('night');
    table.scene.environment = { preset: 'dusk', brightness: 0.55 };
    editor.chooseLight('magic'); click();
    await editor.publish();
    expect(saved.layout.environment).toEqual({ preset: 'dusk', brightness: 0.55 });
  } finally { table.scene = previous; }
});

it('publishes an environment undo and redo after an earlier publish', async () => {
  const { table } = await import('../table.svelte');
  const previous = table.scene;
  let saved = JSON.parse(JSON.stringify(editor.draft));
  vi.stubGlobal('fetch', vi.fn(async (url: string, options?: RequestInit) => {
    if (options?.method === 'PUT') saved = JSON.parse(String(options.body));
    if (url.endsWith('/publish')) saved.environmentBase = { ...(saved.layout.environment ?? DAYLIGHT) };
    return { ok: true, json: async () => JSON.parse(JSON.stringify(saved)) };
  }));
  await editor.load('lighting-test');
  table.scene = { id: 'live', environment: { ...DAYLIGHT } } as NonNullable<typeof table.scene>;
  try {
    editor.setEnvironment({ preset: 'night', brightness: 0.16 });
    await editor.publish();
    table.scene.environment = { preset: 'night', brightness: 0.16 };
    editor.undo();
    await editor.publish();
    expect(saved.layout.environment ?? DAYLIGHT).toEqual(DAYLIGHT);
    table.scene.environment = { ...DAYLIGHT };
    editor.redo();
    await editor.publish();
    expect(saved.layout.environment).toEqual({ preset: 'night', brightness: 0.16 });
  } finally { table.scene = previous; }
});

it('retains an autosaved unpublished environment across draft reload', async () => {
  const { table } = await import('../table.svelte');
  const previous = table.scene;
  let saved = JSON.parse(JSON.stringify(editor.draft));
  vi.stubGlobal('fetch', vi.fn(async (url: string, options?: RequestInit) => {
    if (options?.method === 'PUT') saved = JSON.parse(String(options.body));
    if (url.endsWith('/publish')) saved.environmentBase = { ...(saved.layout.environment ?? DAYLIGHT) };
    return { ok: true, json: async () => JSON.parse(JSON.stringify(saved)) };
  }));
  await editor.load('lighting-test');
  table.scene = { id: 'live', environment: { ...DAYLIGHT } } as NonNullable<typeof table.scene>;
  try {
    editor.setEnvironment({ preset: 'night', brightness: 0.16 });
    await vi.advanceTimersByTimeAsync(800);
    expect(saved.environmentBase).toEqual(DAYLIGHT);
    await editor.load('lighting-test');
    expect(editor.draft!.layout.environment?.preset).toBe('night');
    await editor.publish();
    expect(saved.layout.environment).toEqual({ preset: 'night', brightness: 0.16 });
    expect(saved.environmentBase).toEqual(saved.layout.environment);
    // An unchanged published draft follows a subsequent live change even after loading it again.
    table.scene.environment = { preset: 'dusk', brightness: 0.55 };
    await editor.load('lighting-test');
    editor.chooseLight('torch'); click();
    await editor.publish();
    expect(saved.layout.environment).toEqual({ preset: 'dusk', brightness: 0.55 });
  } finally { table.scene = previous; }
});
