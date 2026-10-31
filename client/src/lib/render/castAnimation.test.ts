import * as THREE from 'three';
import { describe, expect, it } from 'vitest';
import { ANIMATION_KINDS, CastAnimator, CLIP_MS, sampleCastPose, type CastFacing } from './castAnimation';
import { attackAnimation } from './castCues';
import { OcclusionOutlines } from './occlusion';

function fixture() {
  const card = new THREE.Mesh(new THREE.PlaneGeometry(), new THREE.MeshBasicMaterial({ map: new THREE.Texture(), side: THREE.DoubleSide }));
  const animator = new CastAnimator(card);
  animator.setFrame([128, 256], [64, 240], 128, 'south');
  return { card, animator };
}

describe('compact four-view cast animation', () => {
  it('has a distinct, finite pose in each state and direction, with reduced-motion suppression', () => {
    for (const facing of ['north', 'east', 'south', 'west'] as CastFacing[]) {
      const signatures = new Set<string>();
      for (const kind of ANIMATION_KINDS) {
        const pose = new Float32Array(8);
        sampleCastPose(kind, .42, facing, pose);
        expect([...pose].every(Number.isFinite)).toBe(true);
        expect([...pose].some(value => value !== 0)).toBe(true);
        signatures.add([...pose].join(','));
        expect([...sampleCastPose(kind, 1, facing, pose)]).toEqual(Array(8).fill(0));
        expect([...sampleCastPose(kind, .42, facing, pose, true)]).toEqual(Array(8).fill(0));
      }
      expect(signatures.size).toBe(6);
    }
  });

  it('reuses buffers, anchors feet, restores the exact rest pose and stops finite previews', () => {
    const { card, animator } = fixture();
    const position = card.geometry.getAttribute('position') as THREE.BufferAttribute;
    const rest = new Float32Array(position.array);
    for (const kind of ANIMATION_KINDS) {
      animator.play(kind, 100);
      expect(animator.tick(100 + CLIP_MS[kind] * .42, false, true, false, false)).toBe(80);
      expect(position.array).not.toEqual(rest);
      for (let i = 0; i < rest.length; i += 3) if (rest[i + 1]! <= 0) expect([...position.array.slice(i, i + 3)]).toEqual([...rest.slice(i, i + 3)]);
      expect(animator.tick(100 + CLIP_MS[kind], false, true, false, false)).toBe(Infinity);
      expect(position.array).toEqual(rest);
      expect(card.geometry.getAttribute('position')).toBe(position);
      expect(card.children.every(child => !child.visible)).toBe(true);
    }
    animator.dispose(); card.geometry.dispose(); card.material.dispose();
  });

  it('sleeps between idle breaths and disables all continuous motion for reduced-motion users', () => {
    const { card, animator } = fixture();
    expect(animator.tick(6000, false, true, false)).toBe(2000);
    expect(animator.tick(8400, false, true, false)).toBe(80);
    animator.play('spell', 8500);
    expect(animator.tick(8800, true, true, true)).toBe(Infinity);
    expect(card.children.every(child => !child.visible)).toBe(true);
    animator.dispose();
  });

  it('raycasts the deformed mesh and invalidates cached silhouette geometry', () => {
    const { card, animator } = fixture();
    const outlines = new OcclusionOutlines() as unknown as { castChanged: (bodies: THREE.Mesh[], stamp: number) => boolean; dispose: () => void };
    const ray = new THREE.Raycaster(new THREE.Vector3(.62, 1.3, 1), new THREE.Vector3(0, 0, -1));
    card.updateMatrixWorld(true);
    expect(ray.intersectObject(card, false)).toHaveLength(0);
    expect(outlines.castChanged([card], 1)).toBe(true);
    expect(outlines.castChanged([card], 1)).toBe(false);
    animator.play('melee', 0);
    animator.tick(550, false, false, false, false);
    expect(ray.intersectObject(card, false).length).toBeGreaterThan(0);
    expect(outlines.castChanged([card], 1)).toBe(true);
    expect(outlines.castChanged([card], 1)).toBe(false);
    animator.clear();
    expect(ray.intersectObject(card, false)).toHaveLength(0);
    expect(outlines.castChanged([card], 1)).toBe(true);
    animator.dispose(); outlines.dispose();
  });

  it('uses weapon range and covers visible starter adversaries without private sheets', () => {
    expect(attackAnimation({ texts: { attackrange: 'Far' } })).toBe('ranged');
    expect(attackAnimation({ texts: { primary: 'Sword · Strength · Melee' } })).toBe('melee');
    expect(attackAnimation({ texts: { attackrange: 'Very Close' } })).toBe('melee');
    expect(attackAnimation(undefined, 'forest/bandit-archer')).toBe('ranged');
    expect(attackAnimation(undefined, 'forest/elf')).toBe('ranged');
    expect(attackAnimation(undefined, 'forest/bandit-captain')).toBe('melee');
  });
});
