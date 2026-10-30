// ------------------------------------------------------------
// Module: decals
// Purpose: Flat ground details on a built map. Without a picture
//          they stay a faint mark, so a scatter is still visible.
// ------------------------------------------------------------

import * as THREE from 'three';

import { fileUrl, findIn, loadTexture, type Pack } from '../assets/catalog';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { seen } from './seen';

export class DecalLayer {
  private group = new THREE.Group();
  private key = '';

  constructor(scene: THREE.Scene) {
    scene.add(this.group);
  }

  sync(layout: MapLayout | null, packs: Pack[], fog: FogView | null, showAll: boolean, onPaint: () => void = () => {}) {
    const key = layout ? `${layout.decals.map((decal) => `${decal.asset}@${decal.x},${decal.y},${decal.scale},${decal.angle}`).join(';')}:${showAll}:${fog?.mode ?? ''}:${fog?.explored ?? ''}` : '';
    if (key === this.key) return;
    this.key = key;
    clear(this.group);
    if (!layout) return;
    for (const decal of layout.decals) {
      if (!showAll && !seen(fog, decal.x, decal.y)) continue;
      const found = findIn(packs, decal.asset, (pack) => pack.decals);
      const overlay = findIn(packs, decal.asset, (pack) => (pack.tiles ?? []).filter((tile) => tile.kind === 'overlay'));
      const width = (overlay?.def.coverageCells[0] ?? found?.def.size[0] ?? 1) * (decal.scale || 1);
      const depth = (overlay?.def.coverageCells[1] ?? found?.def.size[1] ?? 1) * (decal.scale || 1);
      const mesh = new THREE.Mesh(
        new THREE.PlaneGeometry(width, depth),
        new THREE.MeshBasicMaterial({ color: 0xc6b18a, transparent: true, opacity: 0.45, depthWrite: false }),
      );
      mesh.rotation.x = -Math.PI / 2;
      mesh.rotation.z = -THREE.MathUtils.degToRad(decal.angle || 0);
      mesh.position.set(decal.x, 0.02, decal.y);
      mesh.renderOrder = 1;
      this.group.add(mesh);
      const source = overlay ? { pack: overlay.pack, file: overlay.def.files[0] } : found ? { pack: found.pack, file: found.def.file } : null;
      const url = source ? fileUrl(source.pack, source.file) : null;
      if (!url || !source) continue;
      void loadTexture(url, source.pack.filtering).then((texture) => {
        if (!texture || !mesh.parent) return;
        const material = mesh.material as THREE.MeshBasicMaterial;
        material.map = texture;
        material.color.set(0xffffff);
        material.opacity = 1;
        material.alphaTest = 0.05;
        material.needsUpdate = true;
        onPaint();
      });
    }
  }
}

function clear(group: THREE.Group) {
  for (const child of [...group.children]) {
    if (child instanceof THREE.Mesh) {
      child.geometry.dispose();
      const material = child.material as THREE.MeshBasicMaterial;
      material.map = null;
      material.dispose();
    }
    child.removeFromParent();
  }
}
