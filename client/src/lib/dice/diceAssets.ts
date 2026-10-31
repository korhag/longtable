import * as THREE from 'three';
import type { DieMesh } from '../net/bindings/DieMesh';
import { d4CornerLabels } from './faceLabels';
import { dieLook, type TagColor } from './palette';

const CELL = 128;
interface FaceLabel { value: number; side: number; center: THREE.Vector3; rotation: THREE.Quaternion }
interface GeometrySet { body: THREE.BufferGeometry; labels: THREE.BufferGeometry; faces: FaceLabel[]; signature: string }

/** A body and a single batched label mesh replace one draw call per numbered
 * face. Six shapes and eight warm atlases are retained by this tray only.
 */
export class DiceAssets {
  private shapes = new Map<number, GeometrySet>();
  private atlases = new Map<string, THREE.Texture>();
  private activeAtlases = new Set<string>();

  beginRoll() { this.activeAtlases.clear(); }

  build(mesh: DieMesh, tag: string | null, tags: TagColor[]) {
    const signature = JSON.stringify(mesh);
    let shape = this.shapes.get(mesh.sides);
    if (!shape || shape.signature !== signature) {
      shape?.body.dispose(); shape?.labels.dispose();
      shape = { ...diceGeometry(mesh), signature };
      this.shapes.set(mesh.sides, shape);
    }
    const look = dieLook(mesh.sides, tag, tags);
    const key = `${mesh.sides}|${look.ink}|${look.stroke}`;
    let atlas = this.atlases.get(key);
    if (!atlas) {
      atlas = numberAtlas(mesh.sides, look.ink, look.stroke);
      if (this.atlases.size >= 8) {
        const idle = [...this.atlases.keys()].find(candidate => !this.activeAtlases.has(candidate));
        if (idle) { this.atlases.get(idle)?.dispose(); this.atlases.delete(idle); }
      }
      // An unusually colorful roll can exceed the warm-cache size. Its extra
      // textures belong to that roll and are freed rather than growing the cache.
      if (this.atlases.size < 8) this.atlases.set(key, atlas);
    } else { this.atlases.delete(key); this.atlases.set(key, atlas); }
    this.activeAtlases.add(key);
    const group = new THREE.Group();
    group.add(new THREE.Mesh(shape.body, new THREE.MeshLambertMaterial({ color: look.bodyHex })));
    const labels = new THREE.Mesh(shape.labels, new THREE.MeshBasicMaterial({ map: atlas, transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -2 }));
    labels.renderOrder = 2;
    group.add(labels);
    group.userData.labelFaces = shape.faces;
    return group;
  }

  highlight(group: THREE.Group, value: number) {
    const positions: number[] = [];
    for (const face of (group.userData.labelFaces ?? []) as FaceLabel[]) {
      if (face.value !== value) continue;
      const indexed = new THREE.RingGeometry(face.side * .46, face.side * .58, 24);
      const ring = indexed.toNonIndexed();
      indexed.dispose();
      ring.applyQuaternion(face.rotation);
      const normal = new THREE.Vector3(0, 0, 1).applyQuaternion(face.rotation);
      ring.translate(face.center.x + normal.x * .012, face.center.y + normal.y * .012, face.center.z + normal.z * .012);
      positions.push(...ring.getAttribute('position').array);
      ring.dispose();
    }
    if (!positions.length) return;
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
    const halo = new THREE.Mesh(geometry, new THREE.MeshBasicMaterial({ color: 0xf2d48a, side: THREE.DoubleSide, transparent: true, depthWrite: false }));
    halo.renderOrder = 3;
    group.add(halo);
  }

  release(objects: THREE.Object3D[]) {
    const warmGeometry = new Set([...this.shapes.values()].flatMap(shape => [shape.body, shape.labels]));
    const warmTextures = new Set(this.atlases.values());
    disposeObjects(objects, warmGeometry, warmTextures);
  }

  dispose() {
    for (const shape of this.shapes.values()) { shape.body.dispose(); shape.labels.dispose(); }
    for (const atlas of this.atlases.values()) atlas.dispose();
    this.shapes.clear(); this.atlases.clear(); this.activeAtlases.clear();
  }
}

export function disposeObjects(objects: THREE.Object3D[], retainedGeometry = new Set<THREE.BufferGeometry>(), retainedTextures = new Set<THREE.Texture>()) {
  const geometry = new Set<THREE.BufferGeometry>(), materials = new Set<THREE.Material>(), textures = new Set<THREE.Texture>();
  for (const object of objects) object.traverse(child => {
    if (child instanceof THREE.Mesh) geometry.add(child.geometry);
    if (!(child instanceof THREE.Mesh || child instanceof THREE.Sprite)) return;
    for (const material of Array.isArray(child.material) ? child.material : [child.material]) {
      materials.add(material);
      const texture = (material as THREE.MeshBasicMaterial).map;
      if (texture) textures.add(texture);
    }
  });
  for (const item of geometry) if (!retainedGeometry.has(item)) item.dispose();
  for (const item of materials) item.dispose();
  for (const item of textures) if (!retainedTextures.has(item)) item.dispose();
}

export function diceGeometry(mesh: DieMesh): Omit<GeometrySet, 'signature'> {
  const bodyPositions: number[] = [], normals: number[] = [];
  for (const face of mesh.faces) {
    for (let corner = 1; corner < face.indices.length - 1; corner++) {
      for (const index of [face.indices[0]!, face.indices[corner]!, face.indices[corner + 1]!]) {
        const vertex = mesh.vertices[index];
        if (!vertex) throw new Error('Die mesh has an invalid vertex');
        bodyPositions.push(...vertex); normals.push(...face.normal);
      }
    }
  }
  const body = new THREE.BufferGeometry();
  body.setAttribute('position', new THREE.Float32BufferAttribute(bodyPositions, 3));
  body.setAttribute('normal', new THREE.Float32BufferAttribute(normals, 3));
  const faces: FaceLabel[] = [];
  const plate = (faceIndex: number, value: number, corner?: number) => {
    const face = mesh.faces[faceIndex]!;
    const points = face.indices.map(index => new THREE.Vector3(...mesh.vertices[index]!));
    const center = points.reduce((sum, point) => sum.add(point), new THREE.Vector3()).multiplyScalar(1 / points.length);
    let radius = Infinity;
    for (let i = 0; i < points.length; i++) {
      const a = points[i]!, edge = points[(i + 1) % points.length]!.clone().sub(a);
      const t = THREE.MathUtils.clamp(center.clone().sub(a).dot(edge) / (edge.lengthSq() || 1), 0, 1);
      radius = Math.min(radius, center.distanceTo(a.clone().addScaledVector(edge, t)));
    }
    const normal = new THREE.Vector3(...face.normal).normalize();
    let up = new THREE.Vector3(0, 1, 0);
    if (corner !== undefined) { up = new THREE.Vector3(...mesh.vertices[corner]!).sub(center); center.addScaledVector(up, .58); }
    center.addScaledVector(normal, corner === undefined ? .014 : .016);
    const rotation = new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 0, 1), normal);
    const currentUp = new THREE.Vector3(0, 1, 0).applyQuaternion(rotation).projectOnPlane(normal);
    up.projectOnPlane(normal);
    if (currentUp.lengthSq() > 1e-8 && up.lengthSq() > 1e-8) {
      currentUp.normalize(); up.normalize();
      const twist = Math.atan2(currentUp.clone().cross(up).dot(normal), currentUp.dot(up));
      rotation.premultiply(new THREE.Quaternion().setFromAxisAngle(normal, twist));
    }
    faces.push({ value, center, rotation, side: Math.max(.05, radius * (corner === undefined ? 2.1 : 1.05)) });
  };
  if (mesh.sides === 4) for (const label of d4CornerLabels(mesh)) plate(label.face, label.value, label.vertex);
  else mesh.faces.forEach((face, index) => plate(index, face.value));
  const positions: number[] = [], uvs: number[] = [];
  const columns = Math.ceil(Math.sqrt(mesh.sides)), rows = Math.ceil(mesh.sides / columns);
  for (const face of faces) {
    const col = (face.value - 1) % columns, row = Math.floor((face.value - 1) / columns);
    for (const [u, v] of [[0, 1], [0, 0], [1, 1], [0, 0], [1, 0], [1, 1]]) {
      const point = new THREE.Vector3((u! - .5) * face.side, (v! - .5) * face.side, 0).applyQuaternion(face.rotation).add(face.center);
      positions.push(point.x, point.y, point.z);
      uvs.push((col * CELL + .5 + u! * (CELL - 1)) / (columns * CELL), ((rows - row - 1) * CELL + .5 + v! * (CELL - 1)) / (rows * CELL));
    }
  }
  const labels = new THREE.BufferGeometry();
  labels.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  labels.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2));
  return { body, labels, faces };
}

function numberAtlas(sides: number, ink: string, stroke: string) {
  const canvas = document.createElement('canvas');
  const columns = Math.ceil(Math.sqrt(sides)), rows = Math.ceil(sides / columns);
  canvas.width = columns * CELL; canvas.height = rows * CELL;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('Dice labels need a canvas');
  context.textAlign = 'center'; context.textBaseline = 'middle'; context.lineJoin = 'round';
  for (let value = 1; value <= sides; value++) {
    const x = ((value - 1) % columns + .5) * CELL, y = (Math.floor((value - 1) / columns) + .527) * CELL;
    let size = CELL * .78;
    context.font = `600 ${size}px Inter, sans-serif`;
    const width = context.measureText(String(value)).width;
    if (width > CELL * .87) size *= CELL * .87 / width;
    context.font = `600 ${size}px Inter, sans-serif`;
    context.lineWidth = CELL * .055; context.strokeStyle = stroke; context.fillStyle = ink;
    context.strokeText(String(value), x, y); context.fillText(String(value), x, y);
    if (sides >= 9 && (value === 6 || value === 9)) {
      context.strokeStyle = ink; context.lineWidth = CELL * .035;
      context.beginPath(); context.moveTo(x - CELL * .18, y + size * .34); context.lineTo(x + CELL * .18, y + size * .34); context.stroke();
    }
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.generateMipmaps = false; texture.minFilter = texture.magFilter = THREE.LinearFilter;
  return texture;
}
