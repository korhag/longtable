// ------------------------------------------------------------
// Module: sceneObjects
// Purpose: Schema-2 objects as retained cutouts. One door or one
//          fog cell changes visibility or that object's picture.
//          The other objects keep the same meshes.
// ------------------------------------------------------------

import * as THREE from 'three';

import { fileUrl, findIn, loadTexture, type ObjectDef, type ObjectLayer, type Pack } from '../assets/catalog';
import type { DoorView } from '../net/bindings/DoorView.ts';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { CUTOUT_RIGHT, CUTOUT_TOWARD, CUTOUT_UP, cutoutMaterial, opaqueTexel, orientUpright, uprightGeometry } from './cutout';
import { structureGeometry, structureTexture } from './architecture';
import { cellLevel, SUPPORT_CLEARANCE, supportHeight } from './levels';
import { seen } from './seen';
import { setWallCutaway } from './wallCutaway';

export const objectStats = { rebuilds: 0, instances: 0 };

type Node = { sig: string; group: THREE.Group };
type Batch = { sig: string; capacity: number; mesh: THREE.InstancedMesh; material: THREE.MeshBasicMaterial; geometry: THREE.BufferGeometry };

const uprightQuat = new THREE.Quaternion().setFromRotationMatrix(new THREE.Matrix4().makeBasis(CUTOUT_RIGHT, CUTOUT_UP, CUTOUT_TOWARD));
const groundQuat = new THREE.Quaternion().setFromEuler(new THREE.Euler(-Math.PI / 2, 0, 0));
const scratchPosition = new THREE.Vector3();
const scratchScale = new THREE.Vector3(1, 1, 1);
const hiddenScale = new THREE.Vector3(0, 0, 0);
const scratchMatrix = new THREE.Matrix4();

export class SceneObjects {
  private group = new THREE.Group();
  private nodes = new Map<string, Node>();
  private batches = new Map<string, Batch>();
  private architecture = new Map<string, THREE.BufferGeometry>();
  private generation = 0;

  constructor(scene: THREE.Scene) {
    this.group.renderOrder = 3;
    scene.add(this.group);
  }

  sync(layout: MapLayout | null, packs: Pack[], doors: DoorView[], fog: FogView | null, showAll: boolean, onPaint: () => void, lowered: Set<string> = new Set()) {
    const generation = this.generation;
    const wanted = new Map<string, { sig: string; visible: boolean; door: boolean; make: () => THREE.Group }>();
    const placed = new Map<string, { sig: string; matrices: THREE.Matrix4[]; layer: ObjectLayer; pack: Pack; pixels: number }>();
    if (layout) {
      for (const prop of layout.props) {
        const found = findIn(packs, prop.asset, (pack) => pack.objects ?? []);
        if (!found) continue;
        const origin = found.def.originCells ?? [found.def.footprintCells[0] / 2, found.def.footprintCells[1] / 2];
        const x = prop.x + origin[0];
        const z = prop.y + origin[1];
        const height = supportHeight(layout, x, z);
        const visible = showAll || seen(fog, x, z);
        found.def.renderLayers.forEach((layer, index) => {
          const key = `${prop.asset}:${index}:${layer.file}:${layer.kind}:${found.def.pixelsPerWorldUnit ?? 128}:${found.pack.contentHash ?? ''}`;
          const bucket = placed.get(key) ?? { sig: '', matrices: [], layer, pack: found.pack, pixels: found.def.pixelsPerWorldUnit ?? 128 };
          bucket.matrices.push(instanceMatrix(x, height, z, layer, visible));
          bucket.sig += `${prop.id}@${x},${height},${z},${visible ? 1 : 0};`;
          placed.set(key, bucket);
        });
      }
      for (const piece of layout.pieces) {
        const found = findIn(packs, piece.asset, (pack) => pack.objects ?? []);
        if (!found || (found.def.kind !== 'wall' && found.def.kind !== 'door')) continue;
        const open = doors.find((door) => door.id === piece.id)?.open ?? piece.open;
        const midX = piece.axis === 'h' ? piece.x + 0.5 : piece.x;
        const midY = piece.axis === 'v' ? piece.y + 0.5 : piece.y;
        const height = edgeLevel(layout, piece.axis, piece.x, piece.y);
        const sig = `${piece.asset}:${piece.axis}:${piece.x},${piece.y}:${open ? 1 : 0}:${height}:${found.pack.contentHash ?? ''}`;
        wanted.set(piece.id, {
          sig,
          visible: showAll || seen(fog, midX, midY),
          door: piece.kind === 'door',
          make: () => this.pieceGroup(piece.axis, piece.x, piece.y, height, open, found.def, found.pack, generation, onPaint),
        });
      }
    }
    this.syncBatches(placed, generation, onPaint);
    for (const [id, node] of this.nodes) {
      if (wanted.has(id)) continue;
      disposeGroup(node.group);
      node.group.removeFromParent();
      this.nodes.delete(id);
    }
    for (const [id, spec] of wanted) {
      const existing = this.nodes.get(id);
      if (existing && existing.sig === spec.sig) {
        existing.group.visible = spec.visible;
        setWallCutaway(existing.group, lowered.has(id));
        continue;
      }
      if (existing) {
        disposeGroup(existing.group);
        existing.group.removeFromParent();
      }
      const group = spec.make();
      if (spec.door) group.userData.doorId = id;
      group.visible = spec.visible;
      setWallCutaway(group, lowered.has(id));
      this.group.add(group);
      this.nodes.set(id, { sig: spec.sig, group });
      objectStats.rebuilds += 1;
    }
  }

  clear() {
    this.generation += 1;
    for (const node of this.nodes.values()) {
      disposeGroup(node.group);
      node.group.removeFromParent();
    }
    this.nodes.clear();
    for (const batch of this.batches.values()) disposeBatch(batch);
    this.batches.clear();
    for (const geometry of this.architecture.values()) geometry.dispose();
    this.architecture.clear();
  }

  /** Pick visible door artwork or solid architecture, rather than its ground projection. */
  doorAt(raycaster: THREE.Raycaster): string | null {
    let closest: { id: string; distance: number } | null = null;
    for (const [id, node] of this.nodes) {
      if (!node.group.visible || !node.group.userData.doorId) continue;
      node.group.updateWorldMatrix(true, true);
      for (const hit of raycaster.intersectObject(node.group, true)) {
        if (!(hit.object instanceof THREE.Mesh) || !hit.object.visible) continue;
        const material = Array.isArray(hit.object.material) ? hit.object.material[hit.face?.materialIndex ?? 0] : hit.object.material;
        if (!material || (material.clippingPlanes?.some((plane: THREE.Plane) => plane.distanceToPoint(hit.point) < 0))) continue;
        if (hit.uv && material.alphaTest > 0 && !opaqueTexel(material.map?.image, hit.uv.x, hit.uv.y)) continue;
        if (!closest || hit.distance < closest.distance) closest = { id, distance: hit.distance };
        break;
      }
    }
    return closest?.id ?? null;
  }

  private syncBatches(
    placed: Map<string, { sig: string; matrices: THREE.Matrix4[]; layer: ObjectLayer; pack: Pack; pixels: number }>,
    generation: number,
    onPaint: () => void,
  ) {
    for (const [key, batch] of this.batches) {
      if (placed.has(key)) continue;
      disposeBatch(batch);
      this.batches.delete(key);
    }
    for (const [key, spec] of placed) {
      const existing = this.batches.get(key);
      if (existing?.sig === spec.sig) continue;
      if (existing && existing.capacity >= spec.matrices.length) {
        writeInstances(existing.mesh, spec.matrices);
        existing.sig = spec.sig;
        continue;
      }
      if (existing) disposeBatch(existing);
      const geometry = uprightGeometry(spec.layer.sizePx, spec.layer.pivotPx, spec.pixels);
      const material = cutoutMaterial(null, 0xffffff, spec.layer.kind === 'ground');
      const mesh = new THREE.InstancedMesh(geometry, material, Math.max(spec.matrices.length, 1));
      mesh.frustumCulled = false;
      mesh.renderOrder = spec.layer.kind === 'ground' ? 1 : 3;
      writeInstances(mesh, spec.matrices);
      this.group.add(mesh);
      this.batches.set(key, { sig: spec.sig, capacity: mesh.count, mesh, material, geometry });
      objectStats.rebuilds += 1;
      const url = fileUrl(spec.pack, spec.layer.file);
      if (!url) continue;
      void loadTexture(url, spec.pack.filtering).then((texture) => {
        if (!texture || generation !== this.generation || !mesh.parent) return;
        material.map = texture;
        material.needsUpdate = true;
        onPaint();
      });
    }
    let instances = 0;
    for (const batch of this.batches.values()) instances += batch.mesh.count;
    objectStats.instances = instances;
  }

  private pieceGroup(axis: string, x: number, y: number, height: number, open: boolean, object: ObjectDef, pack: Pack, generation: number, onPaint: () => void) {
    const group = new THREE.Group();
    const along = axis === 'h';
    group.position.set(along ? x + 0.5 : x, height + SUPPORT_CLEARANCE, along ? y : y + 0.5);
    if (object.structure) {
      const structure = object.structure;
      const key = `${structure.profile}:${structure.height}:${structure.thickness}:${open ? 1 : 0}`;
      let geometry = this.architecture.get(key);
      if (!geometry) { geometry = structureGeometry(structure, open); this.architecture.set(key, geometry); }
      const material = new THREE.MeshBasicMaterial({ color: 0x84684b, vertexColors: true });
      const mesh = new THREE.Mesh(geometry, material);
      if (!along) mesh.rotation.y = Math.PI / 2;
      group.add(mesh);
      const url = fileUrl(pack, structure.texture);
      if (url) void loadTexture(url, pack.filtering).then(texture => {
        if (!texture || generation !== this.generation || !mesh.parent) return;
        material.map = structureTexture(texture);
        material.color.set(0xffffff);
        material.needsUpdate = true;
        onPaint();
      });
      return group;
    }
    const state = open ? 'open' : 'closed';
    const layers = object.renderLayers.filter((layer) => (layer.axis ?? '') === '' || layer.axis === axis);
    const matched = layers.filter((layer) => (layer.state ?? '') === '' || layer.state === state);
    const chosen = matched.length > 0 ? matched : layers;
    for (const layer of chosen) this.addLayer(group, layer, object.pixelsPerWorldUnit ?? 128, pack, generation, onPaint);
    return group;
  }

  private addLayer(group: THREE.Group, layer: ObjectLayer, pixels: number, pack: Pack, generation: number, onPaint: () => void) {
    const ground = layer.kind === 'ground';
    const geometry = uprightGeometry(layer.sizePx, layer.pivotPx, pixels);
    const material = cutoutMaterial(null, 0xffffff, ground);
    const mesh = new THREE.Mesh(geometry, material);
    const offset = layer.rootOffsetWorld ?? [0, 0, 0];
    if (ground) {
      mesh.rotation.x = -Math.PI / 2;
      mesh.position.set(offset[0], 0.015 + offset[1], offset[2]);
    } else {
      orientUpright(mesh);
      mesh.position.set(offset[0], offset[1], offset[2]);
    }
    mesh.renderOrder = ground ? 1 : 3;
    group.add(mesh);
    const url = fileUrl(pack, layer.file);
    if (!url) return;
    void loadTexture(url, pack.filtering).then((texture) => {
      if (!texture || generation !== this.generation || !mesh.parent) return;
      material.map = texture;
      material.needsUpdate = true;
      onPaint();
    });
  }
}

function disposeGroup(group: THREE.Group) {
  group.traverse((node) => {
    if (!(node instanceof THREE.Mesh)) return;
    if (!node.geometry.userData.sharedArchitecture) node.geometry.dispose();
    const material = node.material as THREE.MeshBasicMaterial;
    material.map = null;
    material.dispose();
  });
}

function edgeLevel(layout: MapLayout, axis: string, x: number, y: number) {
  const here = cellLevel(layout, x, y);
  const other = axis === 'h' ? cellLevel(layout, x, y - 1) : cellLevel(layout, x - 1, y);
  return Math.max(here, other);
}

function instanceMatrix(x: number, height: number, z: number, layer: ObjectLayer, visible: boolean) {
  const offset = layer.rootOffsetWorld ?? [0, 0, 0];
  const ground = layer.kind === 'ground';
  scratchPosition.set(x + offset[0], height + (ground ? 0.015 : 0) + offset[1], z + offset[2]);
  scratchMatrix.compose(scratchPosition, ground ? groundQuat : uprightQuat, visible ? scratchScale : hiddenScale);
  return scratchMatrix.clone();
}

function writeInstances(mesh: THREE.InstancedMesh, matrices: THREE.Matrix4[]) {
  matrices.forEach((matrix, index) => mesh.setMatrixAt(index, matrix));
  mesh.count = matrices.length;
  mesh.instanceMatrix.needsUpdate = true;
}

function disposeBatch(batch: Batch) {
  batch.mesh.removeFromParent();
  batch.geometry.dispose();
  batch.material.map = null;
  batch.material.dispose();
}
