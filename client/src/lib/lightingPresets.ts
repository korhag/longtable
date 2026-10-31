import type { EnvironmentLighting } from './net/bindings/EnvironmentLighting';
import type { LightKind } from './net/bindings/LightKind';

export const ENVIRONMENT_PRESETS: { name: string; value: EnvironmentLighting }[] = [
  { name: 'Day', value: { preset: 'day', brightness: 1 } },
  { name: 'Dusk', value: { preset: 'dusk', brightness: 0.55 } },
  { name: 'Night', value: { preset: 'night', brightness: 0.16 } },
];
export const DAYLIGHT: EnvironmentLighting = { preset: 'day', brightness: 1 };
export const LIGHT_PRESETS: { kind: LightKind; name: string; color: string; range: number; intensity: number; note: string }[] = [
  { kind: 'torch', name: 'Torch', color: '#ffad42', range: 5, intensity: 1.4, note: 'Golden firelight gently breathes and flickers.' },
  { kind: 'magic', name: 'Magic', color: '#79c8ff', range: 5, intensity: 1.3, note: 'A cold blue glow with a slow, gentle pulse.' },
  { kind: 'steady', name: 'Steady', color: '#f2e2b0', range: 6, intensity: 1, note: 'A constant light in the color you choose.' },
];
