import { describe, expect, it } from 'vitest';
import * as THREE from 'three';
import { ambientColor, closestLights, lightPulse, SceneLighting, type SceneLight } from './lighting';

describe('scene illumination', () => {
  it('leaves old scenes unchanged in daylight and makes night dim and blue', () => {
    expect(ambientColor().toArray()).toEqual([1, 1, 1]);
    const night = ambientColor({ preset: 'night', brightness: 0.16 });
    expect(night.b).toBeGreaterThan(night.r);
    expect(night.b).toBeLessThan(0.16);
    const a = new SceneLighting(new THREE.Scene()), b = new SceneLighting(new THREE.Scene());
    expect(a.sync({ environment: { preset: 'night', brightness: 0.16 } }, null, true)).toBe(true);
    expect(a.sync({ environment: { preset: 'night', brightness: 0.16 } }, null, true)).toBe(false);
    expect(a.sync({ environment: { preset: 'day', brightness: 1 } }, null, true)).toBe(true);
    expect(b.uniforms.sceneAmbient.value.toArray()).toEqual([1, 1, 1]);
    a.dispose(); b.dispose();
  });
  it('gently flickers torch range and strength, with stable reduced-motion and steady modes', () => {
    const light: SceneLight = { x: 4, y: 7, range: 5, color: '#ffad42', kind: 'torch', intensity: 1.4 };
    const values = Array.from({ length: 100 }, (_, i) => lightPulse(light, i / 10));
    expect(new Set(values.map(value => value.range)).size).toBeGreaterThan(1);
    for (const value of values) { expect(value.range).toBeGreaterThanOrEqual(5 * 0.935); expect(value.range).toBeLessThanOrEqual(5 * 1.065); }
    expect(lightPulse(light, 10, true)).toEqual(lightPulse(light, 20, true));
    expect(lightPulse({ ...light, kind: 'steady' }, 10)).toEqual({ range: 5, intensity: 1.4 });
  });
  it('bounds shader work and selects nearby sources deterministically', () => {
    const lights = Array.from({ length: 80 }, (_, x) => ({ x, y: 1, range: 3, color: '#ffffff' }));
    const selected = closestLights(lights, { x: 79, z: 1 });
    expect(selected).toHaveLength(32);
    expect(selected[0]).toBe(lights[79]);
    expect(selected).not.toContain(lights[0]);
  });
});
