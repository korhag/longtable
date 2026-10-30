// ------------------------------------------------------------
// Module: decals
// Purpose: Flat ground details on a built map. A rug sits on the
//          square's own height, above the paint and level with
//          the old floor offset. A fog change hides or shows it.
//          It does not rebuild the other rugs.
// ------------------------------------------------------------

import * as THREE from 'three';

import { fileUrl, findIn, loadTexture, type Pack } from '../assets/catalog';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { PlacedDecal } from '../net/bindings/PlacedDecal.ts';
import { supportHeight } from './levels';
import { seen } from './seen';

type Node = { sig: string; mesh: THREE.Mesh };

export class DecalLayer {
  private group = new THREE.Group();
  private nodes = new Map<string, Node>();
  private stamp = 0;
  private onPaint: () => void = () => {};

  constructor(scene: THREE.Scene) {
    scene.add(this.group);
  }

  sync(layout: MapLayout | null, packs: Pack[], fog: FogView | null, showAll: boolean, onPaint: () => void = () => {}) {
    this.onPaint = onPaint;
    const wanted = new Map<string, { sig: string; visible: boolean; make: () => THREE.Mesh }>();
    const taken = new Map<string, number>();
    if (layout) {
      for (const decal of layout.decals) {
        const height = supportHeight(layout, decal.x, decal.y);
        const found = findIn(packs, decal.asset, (pack) => pack.decals);
        const overlay = findIn(packs, decal.asset, (pack) => (pack.tiles ?? []).filter((tile) => tile.kind === 'overlay'));
        const source = overlay?.pack ?? found?.pack;
        const base = `${decal.asset}@${decal.x},${decal.y},${decal.scale},${decal.angle},${height}:${source?.contentHash ?? ''}`;
        const count = taken.get(base) ?? 0;
        taken.set(base, count + 1);
        const id = count === 0 ? base : `${base}#${count}`;
        wanted.set(id, {
          sig: base,
          visible: showAll || seen(fog, decal.x, decal.y),
          make: () => this.meshFor(decal, packs, height),
        });
      }
    }
    for (const [id, node] of this.nodes) {
      if (wanted.has(id)) continue;
      disposeMesh(node.mesh);
      node.mesh.removeFromParent();
      this.nodes.delete(id);
    }
    for (const [id, spec] of wanted) {
      const existing = this.nodes.get(id);
      if (existing && existing.sig === spec.sig) {
        existing.mesh.visible = spec.visible;
        continue;
      }
      if (existing) {
        disposeMesh(existing.mesh);
        existing.mesh.removeFromParent();
      }
      const mesh = spec.make();
      mesh.visible = spec.visible;
      this.group.add(mesh);
      this.nodes.set(id, { sig: spec.sig, mesh });
    }
  }

  clear() {
    this.stamp += 1;
    for (const node of this.nodes.values()) {
      disposeMesh(node.mesh);
      node.mesh.removeFromParent();
    }
    this.nodes.clear();
  }

  private meshFor(decal: PlacedDecal, packs: Pack[], height: number) {
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
    mesh.position.set(decal.x, height, decal.y);
    mesh.renderOrder = 1;
    const source = overlay ? { pack: overlay.pack, file: overlay.def.files[0] } : found ? { pack: found.pack, file: found.def.file } : null;
    const url = source ? fileUrl(source.pack, source.file) : null;
    if (url && source) {
      this.stamp += 1;
      const stamp = this.stamp;
      mesh.userData.stamp = stamp;
      void loadTexture(url, source.pack.filtering).then((texture) => {
        if (!texture || mesh.userData.stamp !== stamp || !mesh.parent) return;
        const material = mesh.material as THREE.MeshBasicMaterial;
        material.map = texture;
        material.color.set(0xffffff);
        material.opacity = 1;
        material.alphaTest = 0.05;
        material.needsUpdate = true;
        this.onPaint();
      });
    }
    return mesh;
  }
}

function disposeMesh(mesh: THREE.Mesh) {
  mesh.geometry.dispose();
  const material = mesh.material as THREE.MeshBasicMaterial;
  material.map = null;
  material.dispose();
}
