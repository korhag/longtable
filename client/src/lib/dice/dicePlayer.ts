// ------------------------------------------------------------
// Module: dicePlayer
// Purpose: Play the server's dice frames in a small tray.
//          The mesh is built from the same faces the server
//          numbered, so the face you see is the face it read.
// ------------------------------------------------------------

import * as THREE from 'three';

import type { DieMesh } from '../net/bindings/DieMesh.ts';
import type { RollEvent } from '../net/bindings/RollEvent.ts';
import { d4CornerLabels } from './faceLabels';
import { lineUpSlots, type Slot } from './lineup';
import { dieLook, type TagColor } from './palette';
import { sampleFrame, settleTime } from './playback';

const HOLD_MS = 2500;
const GLIDE_S = 0.7;

interface DieActor {
  group: THREE.Group;
  shadow: THREE.Mesh;
  badge: THREE.Sprite | null;
  frames: RollEvent['dice'][number]['keyframes'];
  value: number;
  sign: number;
  letter: string | null;
  slot: Slot;
}

export class DicePlayer {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera = new THREE.PerspectiveCamera(32, 1, 0.1, 80);
  private actors: DieActor[] = [];
  private frameId = 0;
  private playing = false;
  private currentId = '';
  private trayEye = new THREE.Vector3(0, 8, 3);
  private trayAim = new THREE.Vector3();
  private lineupEye = new THREE.Vector3(0, 8, 2);
  private lineupAim = new THREE.Vector3();
  private felt: THREE.MeshLambertMaterial;
  private wood: THREE.MeshLambertMaterial;
  private tagColors: TagColor[] = [];

  constructor(private canvas: HTMLCanvasElement) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));
    this.renderer.setClearColor(0x000000, 0);
    this.scene.add(new THREE.AmbientLight(0xfff4e4, 0.85));
    const sun = new THREE.DirectionalLight(0xfff0d2, 1.15);
    sun.position.set(3, 8, 4);
    this.scene.add(sun);
    this.felt = new THREE.MeshLambertMaterial({ color: 0x1b3a2c });
    this.wood = new THREE.MeshLambertMaterial({ color: 0x5a3a22, side: THREE.DoubleSide });
    const felt = new THREE.Mesh(new THREE.CircleGeometry(14, 48), this.felt);
    felt.rotation.x = -Math.PI / 2;
    this.scene.add(felt);
    const wood = new THREE.Mesh(new THREE.RingGeometry(7.2, 14, 48), this.wood);
    wood.rotation.x = -Math.PI / 2;
    wood.position.y = -0.01;
    this.scene.add(wood);
  }

  setTray(feltUrl: string | null, woodUrl: string | null) {
    const loader = new THREE.TextureLoader();
    if (feltUrl) {
      loader.load(feltUrl, (texture) => {
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.wrapS = THREE.RepeatWrapping;
        texture.wrapT = THREE.RepeatWrapping;
        this.felt.map = texture;
        this.felt.color.set(0xffffff);
        this.felt.needsUpdate = true;
      });
    }
    if (woodUrl) {
      loader.load(woodUrl, (texture) => {
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.wrapS = THREE.RepeatWrapping;
        texture.wrapT = THREE.RepeatWrapping;
        this.wood.map = texture;
        this.wood.color.set(0xffffff);
        this.wood.needsUpdate = true;
      });
    }
  }

  play(
    roll: RollEvent,
    meshes: DieMesh[],
    serverNow: () => number,
    onLand: () => void,
    onDone: () => void,
    tags: TagColor[] = [],
  ): boolean {
    if (this.currentId === roll.id && this.playing) return false;
    this.currentId = roll.id;
    this.stop();
    this.currentId = roll.id;
    this.resize();
    this.tagColors = tags;
    const order = tags.map((tag) => tag.letter);
    const ordered = roll.dice
      .map((die, index) => ({ die, index }))
      .sort((a, b) => tagRank(a.die.tag, order) - tagRank(b.die.tag, order) || a.index - b.index);
    const slots = lineUpSlots(ordered.length);
    this.frameTray(roll.dice);
    this.frameLineup(slots);
    for (const [place, item] of ordered.entries()) {
      const die = item.die;
      const mesh = meshes.find((candidate) => candidate.sides === die.sides);
      const group = mesh ? buildDie(mesh, die.tag, this.tagColors) : new THREE.Group();
      const shadow = new THREE.Mesh(
        new THREE.CircleGeometry(0.42, 20),
        new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.35, depthWrite: false }),
      );
      shadow.rotation.x = -Math.PI / 2;
      shadow.position.y = 0.01;
      this.scene.add(group, shadow);
      this.actors.push({
        group,
        shadow,
        badge: null,
        frames: die.keyframes,
        value: die.value,
        sign: die.sign,
        letter: die.tag,
        slot: slots[place] ?? { x: 0, z: 0 },
      });
    }
    this.playing = true;
    let landed = false;
    let landedAt = 0;
    const restAt = Math.max(0, ...roll.dice.map((die) => settleTime(die.keyframes, roll.keyframeDt)));
    const loop = () => {
      if (!this.playing) return;
      this.frameId = requestAnimationFrame(loop);
      const elapsed = (serverNow() - roll.playAt) / 1000;
      const glide = elapsed >= restAt && elapsed >= 0 ? smoothstep((elapsed - restAt) / GLIDE_S) : 0;
      for (const actor of this.actors) {
        const sample = sampleFrame(glide > 0 ? restAt : elapsed, roll.keyframeDt, actor.frames.length);
        const from = actor.frames[sample.index];
        const to = actor.frames[sample.next];
        if (!from) continue;
        const frac = sample.frac;
        const x = from.p[0] + ((to?.p[0] ?? from.p[0]) - from.p[0]) * frac;
        const y = from.p[1] + ((to?.p[1] ?? from.p[1]) - from.p[1]) * frac;
        const z = from.p[2] + ((to?.p[2] ?? from.p[2]) - from.p[2]) * frac;
        actor.group.position.set(
          x + (actor.slot.x - x) * glide,
          y + Math.sin(glide * Math.PI) * 0.35,
          z + (actor.slot.z - z) * glide,
        );
        const qa = new THREE.Quaternion(from.q[0], from.q[1], from.q[2], from.q[3]);
        const qb = new THREE.Quaternion(to?.q[0] ?? from.q[0], to?.q[1] ?? from.q[1], to?.q[2] ?? from.q[2], to?.q[3] ?? from.q[3]);
        actor.group.quaternion.copy(qa.slerp(qb, frac));
        actor.shadow.position.x = actor.group.position.x;
        actor.shadow.position.z = actor.group.position.z;
        const lift = Math.min(actor.group.position.y, 2);
        const scale = 1.05 - lift * 0.18;
        actor.shadow.scale.setScalar(Math.max(0.35, scale));
      }
      this.applyCamera(glide);
      if (landed) this.placeTags(elapsed - landedAt);
      this.renderer.render(this.scene, this.camera);
      if (!landed && elapsed >= restAt + GLIDE_S) {
        landed = true;
        landedAt = elapsed;
        this.revealFaces();
        this.raiseTags();
        this.placeTags(0);
        const rollId = this.currentId;
        onLand();
        window.setTimeout(() => {
          if (this.playing && this.currentId === rollId) onDone();
        }, HOLD_MS);
      }
    };
    loop();
    return true;
  }

  stop(clear = true) {
    this.playing = false;
    this.currentId = '';
    cancelAnimationFrame(this.frameId);
    if (!clear) return;
    for (const actor of this.actors) {
      disposeObject(actor.group);
      actor.group.removeFromParent();
      actor.shadow.geometry.dispose();
      (actor.shadow.material as THREE.Material).dispose();
      actor.shadow.removeFromParent();
      if (actor.badge) {
        disposeObject(actor.badge);
        actor.badge.removeFromParent();
      }
    }
    this.actors = [];
  }

  dispose() {
    this.stop();
    this.renderer.dispose();
  }

  resize() {
    const width = this.canvas.clientWidth || 640;
    const height = this.canvas.clientHeight || 256;
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
  }

  private frameTray(dice: { keyframes: { p: [number, number, number] }[] }[]) {
    let minX = Infinity;
    let maxX = -Infinity;
    let minZ = Infinity;
    let maxZ = -Infinity;
    let any = false;
    for (const die of dice) {
      const start = die.keyframes.findIndex((frame) => frame.p[1] < 1.2);
      if (start < 0) continue;
      for (const frame of die.keyframes.slice(start)) {
        any = true;
        minX = Math.min(minX, frame.p[0]);
        maxX = Math.max(maxX, frame.p[0]);
        minZ = Math.min(minZ, frame.p[2]);
        maxZ = Math.max(maxZ, frame.p[2]);
      }
    }
    if (!any) {
      minX = -1.5;
      maxX = 1.5;
      minZ = -1.1;
      maxZ = 1.1;
    } else {
      minX -= 1;
      maxX += 1;
      minZ -= 1;
      maxZ += 1;
    }
    const halfW = Math.min(8.5, Math.max((maxX - minX) / 2, 1.5));
    const halfD = Math.min(6.3, Math.max((maxZ - minZ) / 2, 1.1));
    const cx = (minX + maxX) / 2;
    const cz = (minZ + maxZ) / 2;
    const pitch = THREE.MathUtils.degToRad(70);
    const vertical = Math.tan(THREE.MathUtils.degToRad(this.camera.fov) / 2);
    const horizontal = vertical * Math.max(this.camera.aspect, 0.4);
    const dist = Math.max(halfW / horizontal, halfD / vertical / Math.sin(pitch)) * 1.15;
    this.trayAim.set(cx, 0.1, cz);
    this.trayEye.set(cx, Math.sin(pitch) * dist, cz + Math.cos(pitch) * dist);
  }

  private frameLineup(slots: Slot[]) {
    if (slots.length === 0) return;
    let minX = Infinity;
    let maxX = -Infinity;
    let minZ = Infinity;
    let maxZ = -Infinity;
    for (const slot of slots) {
      minX = Math.min(minX, slot.x);
      maxX = Math.max(maxX, slot.x);
      minZ = Math.min(minZ, slot.z);
      maxZ = Math.max(maxZ, slot.z);
    }
    const cx = (minX + maxX) / 2;
    const cz = (minZ + maxZ) / 2;
    const halfW = (maxX - minX) / 2 + 0.9;
    const halfD = (maxZ - minZ) / 2 + 1.15;
    const pitch = THREE.MathUtils.degToRad(76);
    const vertical = Math.tan(THREE.MathUtils.degToRad(this.camera.fov) / 2);
    const horizontal = vertical * Math.max(this.camera.aspect, 0.4);
    const dist = Math.max(halfW / horizontal, halfD / vertical / Math.sin(pitch)) * 1.28;
    this.lineupAim.set(cx, 0.1, cz);
    this.lineupEye.set(cx, Math.sin(pitch) * dist, cz + Math.cos(pitch) * dist);
  }

  private applyCamera(glide: number) {
    this.camera.position.lerpVectors(this.trayEye, this.lineupEye, glide);
    const aim = this.trayAim.clone().lerp(this.lineupAim, glide);
    this.camera.lookAt(aim);
  }

  private revealFaces() {
    for (const actor of this.actors) {
      const rings: THREE.Mesh[] = [];
      actor.group.traverse((child) => {
        if (!(child instanceof THREE.Mesh) || typeof child.userData.value !== 'number') return;
        const material = child.material as THREE.MeshBasicMaterial;
        const wins = child.userData.value === actor.value;
        material.transparent = true;
        material.opacity = wins ? 1 : 0.3;
        if (!wins) return;
        const width = (child.geometry as THREE.PlaneGeometry).parameters?.width ?? 0.35;
        const ring = new THREE.Mesh(
          new THREE.RingGeometry(width * 0.46, width * 0.58, 36),
          new THREE.MeshBasicMaterial({ color: 0xf2d48a, side: THREE.DoubleSide, transparent: true, depthWrite: false }),
        );
        const normal = new THREE.Vector3(0, 0, 1).applyQuaternion(child.quaternion);
        ring.quaternion.copy(child.quaternion);
        ring.position.copy(child.position).addScaledVector(normal, 0.012);
        ring.renderOrder = 3;
        rings.push(ring);
      });
      for (const ring of rings) actor.group.add(ring);
    }
  }

  private raiseTags() {
    for (const actor of this.actors) {
      actor.badge = valueTag(actor.value, actor.sign, actor.letter, this.tagColors);
      actor.badge.scale.set(0.01, 0.01, 1);
      this.scene.add(actor.badge);
    }
  }

  private placeTags(sinceLand: number) {
    const pop = 1 - (1 - Math.min(1, Math.max(0, sinceLand / 0.22))) ** 3;
    const size = Math.max(0.01, 0.46 * pop);
    for (const actor of this.actors) {
      if (!actor.badge) continue;
      actor.badge.position.set(actor.slot.x, 0.04, actor.slot.z + 0.62);
      actor.badge.scale.set(size, size, 1);
    }
  }
}

function tagRank(tag: string | null, order: string[]) {
  if (!tag) return order.length + 1;
  const index = order.indexOf(tag);
  return index === -1 ? order.length : index;
}

function buildDie(mesh: DieMesh, tag: string | null, tags: TagColor[]) {
  const group = new THREE.Group();
  const positions: number[] = [];
  const normals: number[] = [];
  for (const face of mesh.faces) {
    const vertices = face.indices.map((index) => mesh.vertices[index]).filter((vertex) => vertex);
    for (let corner = 1; corner < vertices.length - 1; corner += 1) {
      for (const vertex of [vertices[0], vertices[corner], vertices[corner + 1]]) {
        if (!vertex) continue;
        positions.push(vertex[0], vertex[1], vertex[2]);
        normals.push(face.normal[0], face.normal[1], face.normal[2]);
      }
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geometry.setAttribute('normal', new THREE.Float32BufferAttribute(normals, 3));
  const look = dieLook(mesh.sides, tag, tags);
  group.add(new THREE.Mesh(geometry, new THREE.MeshLambertMaterial({ color: look.bodyHex })));
  const ink = look.ink;
  const stroke = look.stroke;
  if (mesh.sides === 4) {
    for (const label of d4CornerLabels(mesh)) {
      const plate = cornerPlate(mesh, label.face, label.vertex, label.value, ink, stroke);
      if (plate) group.add(plate);
    }
  } else {
    for (const face of mesh.faces) {
      const plate = numberPlate(face.indices, mesh.vertices, face.normal, face.value, ink, stroke, mesh.sides >= 9);
      if (plate) group.add(plate);
    }
  }
  return group;
}

function numberPlate(
  indices: number[],
  vertices: DieMesh['vertices'],
  normal: [number, number, number],
  value: number,
  ink: string,
  stroke: string,
  underline: boolean,
) {
  const points = facePoints(indices, vertices);
  if (points.length < 3) return null;
  const center = average(points);
  const direction = new THREE.Vector3(normal[0], normal[1], normal[2]).normalize();
  const radius = inscribedRadius(points, center);
  const plate = texturedPlate(radius * 2.1, value, ink, stroke, underline);
  plate.position.copy(center).addScaledVector(direction, 0.014);
  alignPlate(plate, direction, new THREE.Vector3(0, 1, 0));
  return plate;
}

function cornerPlate(mesh: DieMesh, faceIndex: number, vertexIndex: number, value: number, ink: string, stroke: string) {
  const face = mesh.faces[faceIndex];
  const vertex = mesh.vertices[vertexIndex];
  if (!face || !vertex) return null;
  const points = facePoints(face.indices, mesh.vertices);
  if (points.length < 3) return null;
  const center = average(points);
  const corner = new THREE.Vector3(vertex[0], vertex[1], vertex[2]);
  const toward = corner.clone().sub(center);
  const direction = new THREE.Vector3(face.normal[0], face.normal[1], face.normal[2]).normalize();
  const radius = inscribedRadius(points, center);
  const plate = texturedPlate(radius * 1.05, value, ink, stroke, false);
  plate.position.copy(center).addScaledVector(toward, 0.58).addScaledVector(direction, 0.016);
  alignPlate(plate, direction, toward);
  return plate;
}

function texturedPlate(side: number, value: number, ink: string, stroke: string, underline: boolean) {
  const texture = paintNumber(value, ink, stroke, underline);
  const plate = new THREE.Mesh(
    new THREE.PlaneGeometry(Math.max(side, 0.05), Math.max(side, 0.05)),
    new THREE.MeshBasicMaterial({
      map: texture ?? undefined,
      transparent: true,
      depthWrite: false,
      polygonOffset: true,
      polygonOffsetFactor: -2,
    }),
  );
  plate.userData.value = value;
  plate.renderOrder = 2;
  return plate;
}

function paintNumber(value: number, ink: string, stroke: string, underline: boolean) {
  const canvas = document.createElement('canvas');
  canvas.width = 512;
  canvas.height = 512;
  const context = canvas.getContext('2d');
  if (!context) return null;
  context.clearRect(0, 0, 512, 512);
  context.textAlign = 'center';
  context.textBaseline = 'middle';
  context.lineJoin = 'round';
  const text = String(value);
  let size = 400;
  context.font = `600 ${size}px Inter, sans-serif`;
  const measured = context.measureText(text).width;
  if (measured > 450) size = Math.floor(size * (450 / measured));
  context.font = `600 ${size}px Inter, sans-serif`;
  context.lineWidth = Math.max(18, Math.round(28 * (size / 320)));
  context.strokeStyle = stroke;
  context.strokeText(text, 256, 270);
  context.fillStyle = ink;
  context.fillText(text, 256, 270);
  if (underline && (value === 6 || value === 9)) {
    const scale = size / 320;
    const underlineY = 270 + 120 * scale;
    const half = 86 * scale;
    context.strokeStyle = ink;
    context.lineWidth = Math.max(12, Math.round(18 * scale));
    context.beginPath();
    context.moveTo(256 - half, underlineY);
    context.lineTo(256 + half, underlineY);
    context.stroke();
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = 8;
  return texture;
}

function valueTag(value: number, sign: number, tag: string | null, tags: TagColor[]) {
  const look = tagLook(sign, tag, tags);
  const canvas = document.createElement('canvas');
  canvas.width = 256;
  canvas.height = 128;
  const context = canvas.getContext('2d');
  const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ transparent: true, depthWrite: false }));
  if (!context) return sprite;
  context.fillStyle = look.body;
  roundRect(context, 18, 16, 220, 96, 28);
  context.fill();
  context.fillStyle = look.ink;
  context.font = '600 72px Inter, sans-serif';
  context.textAlign = 'center';
  context.textBaseline = 'middle';
  context.fillText(sign < 0 ? `−${value}` : String(value), 128, 68);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  sprite.material.map = texture;
  sprite.renderOrder = 4;
  return sprite;
}

function tagLook(sign: number, tag: string | null, tags: TagColor[]) {
  if (tag && tags.some((item) => item.letter === tag)) return dieLook(12, tag, tags);
  if (sign < 0) return { body: '#8d3d32', ink: '#f6efe4' };
  return { body: '#d4a656', ink: '#1c140e' };
}

function roundRect(context: CanvasRenderingContext2D, x: number, y: number, width: number, height: number, radius: number) {
  context.beginPath();
  context.moveTo(x + radius, y);
  context.arcTo(x + width, y, x + width, y + height, radius);
  context.arcTo(x + width, y + height, x, y + height, radius);
  context.arcTo(x, y + height, x, y, radius);
  context.arcTo(x, y, x + width, y, radius);
  context.closePath();
}

function facePoints(indices: number[], vertices: DieMesh['vertices']) {
  return indices
    .map((index) => vertices[index])
    .filter((vertex) => vertex)
    .map((vertex) => new THREE.Vector3(vertex[0], vertex[1], vertex[2]));
}

function average(points: THREE.Vector3[]) {
  const center = new THREE.Vector3();
  for (const point of points) center.add(point);
  return center.multiplyScalar(1 / points.length);
}

function inscribedRadius(points: THREE.Vector3[], center: THREE.Vector3) {
  let best = Infinity;
  for (let index = 0; index < points.length; index += 1) {
    const start = points[index];
    const end = points[(index + 1) % points.length];
    if (!start || !end) continue;
    best = Math.min(best, pointSegmentDistance(center, start, end));
  }
  return Number.isFinite(best) ? best : 0.2;
}

function pointSegmentDistance(point: THREE.Vector3, start: THREE.Vector3, end: THREE.Vector3) {
  const edge = end.clone().sub(start);
  const lengthSq = edge.lengthSq() || 1;
  const t = Math.min(1, Math.max(0, point.clone().sub(start).dot(edge) / lengthSq));
  return point.distanceTo(start.clone().addScaledVector(edge, t));
}

function alignPlate(plate: THREE.Mesh, normal: THREE.Vector3, up: THREE.Vector3) {
  const rotation = new THREE.Quaternion().setFromUnitVectors(new THREE.Vector3(0, 0, 1), normal);
  const currentUp = new THREE.Vector3(0, 1, 0).applyQuaternion(rotation).projectOnPlane(normal);
  const targetUp = up.clone().projectOnPlane(normal);
  if (currentUp.lengthSq() > 1e-8 && targetUp.lengthSq() > 1e-8) {
    currentUp.normalize();
    targetUp.normalize();
    const twist = Math.atan2(currentUp.clone().cross(targetUp).dot(normal), currentUp.dot(targetUp));
    rotation.premultiply(new THREE.Quaternion().setFromAxisAngle(normal, twist));
  }
  plate.quaternion.copy(rotation);
}

function smoothstep(amount: number) {
  const t = Math.min(1, Math.max(0, amount));
  return t * t * (3 - 2 * t);
}

function disposeObject(object: THREE.Object3D) {
  object.traverse((child) => {
    if (child instanceof THREE.Mesh) {
      child.geometry.dispose();
      disposeMaterial(child.material);
    } else if (child instanceof THREE.Sprite) {
      disposeMaterial(child.material);
    }
  });
}

function disposeMaterial(material: THREE.Material | THREE.Material[]) {
  const materials = Array.isArray(material) ? material : [material];
  for (const item of materials) {
    const mapped = item as THREE.Material & { map?: THREE.Texture | null };
    mapped.map?.dispose();
    item.dispose();
  }
}
