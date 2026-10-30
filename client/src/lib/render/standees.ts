// ------------------------------------------------------------
// Module: standees
// Purpose: Props, walls, and doors as cards facing the fixed
//          camera. A missing picture is a box on the footprint
//          with the asset's name.
// ------------------------------------------------------------

import * as THREE from 'three';

import { fileUrl, findIn, loadTexture, type DoorDef, type Pack, type WallDef } from '../assets/catalog';
import type { DoorView } from '../net/bindings/DoorView.ts';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { PlacedProp } from '../net/bindings/PlacedProp.ts';
import { COS_ELEVATION, sceneBasis } from './cameraProfile';
import { seen } from './seen';
import { standeeWorldSize, uprightPixels } from './view';

const { right } = sceneBasis();
const RIGHT = new THREE.Vector3(right.x, right.y, right.z);
const UP = new THREE.Vector3(0, 1, 0);
const TOWARD = new THREE.Vector3().crossVectors(RIGHT, UP).normalize();

export class StandeeLayer {
  private group = new THREE.Group();
  private key = '';
  private generation = 0;
  private onPaint: () => void = () => {};

  constructor(scene: THREE.Scene) {
    this.group.renderOrder = 3;
    scene.add(this.group);
  }

  sync(layout: MapLayout | null, packs: Pack[], doors: DoorView[], fog: FogView | null, showAll: boolean, onPaint: () => void = () => {}) {
    this.onPaint = onPaint;
    const doorKey = doors.map((door) => `${door.id}:${door.open ? 1 : 0}`).join(',');
    const key = layout ? `${layout.pieces.length}:${JSON.stringify(layout.pieces)}:${JSON.stringify(layout.props)}:${doorKey}:${showAll}:${fog?.explored ?? ''}` : '';
    if (key === this.key) return;
    this.key = key;
    this.generation += 1;
    clear(this.group);
    if (!layout) return;
    const generation = this.generation;
    for (const prop of layout.props) {
      if (findIn(packs, prop.asset, (pack) => pack.objects ?? [])) continue;
      if (findIn(packs, prop.asset, (pack) => pack.objects ?? [])) continue;
      if (!showAll && !seen(fog, prop.x + 0.5, prop.y + 0.5)) continue;
      this.addProp(prop, packs, generation);
    }
    for (const piece of layout.pieces) {
      const object = findIn(packs, piece.asset, (pack) => pack.objects ?? []);
      if (object && (object.def.kind === 'wall' || object.def.kind === 'door')) continue;
      if (findIn(packs, piece.asset, (pack) => pack.objects ?? [])) continue;
      const midX = piece.axis === 'h' ? piece.x + 0.5 : piece.x;
      const midY = piece.axis === 'v' ? piece.y + 0.5 : piece.y;
      if (!showAll && !seen(fog, midX, midY)) continue;
      this.addPiece(piece, packs, doors, generation);
    }
    this.addPosts(layout, packs, showAll, fog, generation);
  }

  private addProp(prop: PlacedProp, packs: Pack[], generation: number) {
    const found = findIn(packs, prop.asset, (pack) => pack.props);
    const footprint = found?.def.footprint ?? [1, 1];
    const height = found?.def.height ?? 1;
    const name = found?.def.name ?? prop.asset;
    const anchor = new THREE.Vector3(prop.x, 0, prop.y + footprint[1]);
    const box = fallback(footprint[0], footprint[1], height, name, found?.def.blocks === 'sight' ? '#8a6233' : '#d4a656');
    box.position.set(prop.x + footprint[0] / 2, height / 2, prop.y + footprint[1] / 2);
    this.group.add(box);
    const url = found ? fileUrl(found.pack, found.def.file) : null;
    if (!url || !found) return;
    const mirror = prop.mirror && !!found.def.mirror;
    void this.swap(url, found.pack.filtering, found.pack.pixelsPerCell, footprint[0], footprint[1], height, found.def.anchor ?? null, anchor, mirror, box, generation, found.pack.pixelsPerCell);
  }

  private addPiece(piece: MapLayout['pieces'][number], packs: Pack[], doors: DoorView[], generation: number) {
    const open = doors.find((door) => door.id === piece.id)?.open ?? piece.open;
    const wall = piece.kind === 'wall' ? findIn(packs, piece.asset, (pack) => pack.walls) : null;
    const door = piece.kind === 'door' ? findIn(packs, piece.asset, (pack) => pack.doors) : null;
    const pack = wall?.pack ?? door?.pack;
    const pixels = pack?.pixelsPerCell ?? 128;
    const along = piece.axis === 'h';
    const footprint: [number, number] = along ? [1, 0.25] : [0.25, 1];
    const anchor = along
      ? new THREE.Vector3(piece.x, 0, piece.y + 0.125)
      : new THREE.Vector3(piece.x - 0.125, 0, piece.y + 1);
    const box = fallback(footprint[0], footprint[1], 1, door?.def.name ?? wall?.def.name ?? piece.asset, open ? '#c6a15a' : '#8d3d32');
    box.position.set(anchor.x + footprint[0] / 2, 0.5, anchor.z - footprint[1] / 2);
    this.group.add(box);
    const url = pieceUrl(piece.axis, open, wall?.def, door?.def, pack ?? null);
    if (!url || !pack) return;
    void this.swap(url, pack.filtering, pixels, footprint[0], footprint[1], 1, null, anchor, false, box, generation, pixels);
  }

  private addPosts(layout: MapLayout, packs: Pack[], showAll: boolean, fog: FogView | null, generation: number) {
    const corners = new Map<string, string>();
    for (const horizontal of layout.pieces) {
      if (horizontal.axis !== 'h' || horizontal.kind !== 'wall') continue;
      for (const vertical of layout.pieces) {
        if (vertical.axis !== 'v' || vertical.kind !== 'wall' || vertical.asset !== horizontal.asset) continue;
        if (vertical.x !== horizontal.x && vertical.x !== horizontal.x + 1) continue;
        if (horizontal.y !== vertical.y && horizontal.y !== vertical.y + 1) continue;
        corners.set(`${vertical.x},${horizontal.y}`, horizontal.asset);
      }
    }
    for (const [key, asset] of corners) {
      const [xText, yText] = key.split(',');
      const x = Number(xText);
      const y = Number(yText);
      if (!showAll && !seen(fog, x, y)) continue;
      const found = findIn(packs, asset, (pack) => pack.walls);
      const anchor = new THREE.Vector3(x - 0.125, 0, y + 0.125);
      const box = fallback(0.25, 0.25, 1, '', '#8d6a45');
      box.position.set(x, 0.5, y);
      this.group.add(box);
      const url = found ? fileUrl(found.pack, found.def.post) : null;
      if (!url || !found) continue;
      void this.swap(url, found.pack.filtering, found.pack.pixelsPerCell, 0.25, 0.25, 1, null, anchor, false, box, generation, found.pack.pixelsPerCell);
    }
  }

  private async swap(
    url: string,
    filtering: Pack['filtering'],
    pixels: number,
    width: number,
    depth: number,
    height: number,
    anchorPx: [number, number] | null,
    anchor: THREE.Vector3,
    mirror: boolean,
    box: THREE.Object3D,
    generation: number,
    packPixels: number,
  ) {
    const texture = await loadTexture(url, filtering);
    if (!texture || generation !== this.generation || !box.parent) return;
    const image = texture.image as { width?: number; height?: number } | undefined;
    const expected = uprightPixels(width, depth, height, packPixels);
    const imageWidth = image?.width || expected.width;
    const imageHeight = image?.height || expected.height;
    const size = standeeWorldSize(imageWidth, imageHeight, pixels);
    const point = anchorPx ?? [0, imageHeight];
    const geometry = new THREE.PlaneGeometry(size.width, size.height);
    geometry.translate((size.width / 2) - point[0] / pixels, point[1] / (pixels * COS_ELEVATION) - size.height / 2, 0);
    const material = new THREE.MeshBasicMaterial({ map: texture, transparent: true, alphaTest: 0.35, depthWrite: true });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.quaternion.setFromRotationMatrix(new THREE.Matrix4().makeBasis(RIGHT, UP, TOWARD));
    mesh.position.copy(anchor);
    mesh.scale.x = mirror ? -1 : 1;
    mesh.renderOrder = 3;
    box.parent?.add(mesh);
    disposeNode(box);
    box.removeFromParent();
    this.onPaint();
  }

  clear() {
    this.generation += 1;
    clear(this.group);
    this.key = '';
  }
}

function pieceUrl(axis: 'h' | 'v', open: boolean, wall: WallDef | undefined, door: DoorDef | undefined, pack: Pack | null) {
  if (!pack) return null;
  if (door) {
    if (axis === 'h') return fileUrl(pack, open ? door.frontOpen : door.frontClosed);
    return fileUrl(pack, open ? door.sideOpen : door.sideClosed);
  }
  if (!wall) return null;
  return fileUrl(pack, axis === 'h' ? wall.front : wall.side);
}

function fallback(width: number, depth: number, height: number, name: string, color: string) {
  const group = new THREE.Group();
  const mesh = new THREE.Mesh(
    new THREE.BoxGeometry(Math.max(width, 0.05), Math.max(height, 0.2), Math.max(depth, 0.05)),
    new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.28, wireframe: true }),
  );
  group.add(mesh);
  if (name) {
    const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: label(name), transparent: true, depthWrite: false }));
    sprite.position.y = height / 2 + 0.35;
    sprite.scale.set(1.4, 0.4, 1);
    group.add(sprite);
  }
  return group;
}

function label(name: string) {
  const canvas = document.createElement('canvas');
  canvas.width = 256;
  canvas.height = 64;
  const context = canvas.getContext('2d');
  if (!context) return null;
  context.font = '600 28px Inter, sans-serif';
  context.fillStyle = '#f6efe4';
  context.textAlign = 'center';
  context.textBaseline = 'middle';
  context.fillText(name, 128, 32);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function clear(group: THREE.Group) {
  for (const child of [...group.children]) {
    disposeNode(child);
    child.removeFromParent();
  }
}

function disposeNode(object: THREE.Object3D) {
  object.traverse((node) => {
    if (node instanceof THREE.Mesh) {
      node.geometry.dispose();
      const material = node.material as THREE.MeshBasicMaterial;
      if (material.map && material.map.image instanceof HTMLCanvasElement) material.map.dispose();
      material.dispose();
    } else if (node instanceof THREE.Sprite) {
      node.material.map?.dispose();
      node.material.dispose();
    }
  });
}
