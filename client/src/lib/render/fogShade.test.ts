import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { setSceneryColor, shadeScenery } from './fogShade';

describe('remembered upright scenery', () => {
  it('dims retained geometry without accumulating darkness and restores its original tint', () => {
    const material = new THREE.MeshBasicMaterial({ color: '#b49765' });
    const original = material.color.clone();
    const group = new THREE.Group().add(new THREE.Mesh(new THREE.BoxGeometry(), material));
    shadeScenery(group, 0.065);
    expect(material.color.r).toBeCloseTo(original.r * 0.065);
    shadeScenery(group, 0.065);
    expect(material.color.r).toBeCloseTo(original.r * 0.065);
    shadeScenery(group, 1);
    expect(material.color.equals(original)).toBe(true);
  });

  it('keeps a late-loaded wall texture in darkness until the player can see it', () => {
    const material = new THREE.MeshBasicMaterial({ color: '#b49765' });
    const group = new THREE.Group().add(new THREE.Mesh(new THREE.BoxGeometry(), material));
    shadeScenery(group, 0.065);
    setSceneryColor(material, 0xffffff);
    shadeScenery(group, group.userData.fogBrightness);
    expect(material.color.r).toBeCloseTo(0.065);
    shadeScenery(group, 1);
    expect(material.color.r).toBe(1);
  });
});
