// ------------------------------------------------------------
// Module: dicePlayer
// Purpose: Play the server's dice frames in a small tray.
//          The mesh is built from the same faces the server
//          numbered, so the face you see is the face it read.
// ------------------------------------------------------------

import * as THREE from 'three';

import type { DieMesh } from '../net/bindings/DieMesh.ts';
import type { RollEvent } from '../net/bindings/RollEvent.ts';
import { DiceAssets, disposeObjects } from './diceAssets';
import { lineUpSlots, type Slot } from './lineup';
import { dieLook, type TagColor } from './palette';
import { ROLL_SPEED, sampleFrame, settleTime } from './playback';

const GLIDE_S = 0.4;
const TAG_POP_S = 0.18;

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
  private cameraAim = new THREE.Vector3();
  private fromRotation = new THREE.Quaternion();
  private toRotation = new THREE.Quaternion();
  private felt: THREE.MeshLambertMaterial;
  private wood: THREE.MeshLambertMaterial;
  private tagColors: TagColor[] = [];
  private assets = new DiceAssets();
  private resume: (() => void) | null = null;
  private disposed = false;
  private visibilityChanged = () => {
    cancelAnimationFrame(this.frameId);
    this.frameId = 0;
    if (!document.hidden) this.resume?.();
  };

  constructor(private canvas: HTMLCanvasElement) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: false, alpha: true, powerPreference: 'low-power' });
    document.addEventListener('visibilitychange', this.visibilityChanged);
    this.renderer.setPixelRatio(1);
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
        if (this.disposed) { texture.dispose(); return; }
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.wrapS = THREE.RepeatWrapping;
        texture.wrapT = THREE.RepeatWrapping;
        this.felt.map?.dispose();
        this.felt.map = texture;
        this.felt.color.set(0xffffff);
        this.felt.needsUpdate = true;
        this.redrawResting();
      });
    }
    if (woodUrl) {
      loader.load(woodUrl, (texture) => {
        if (this.disposed) { texture.dispose(); return; }
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.wrapS = THREE.RepeatWrapping;
        texture.wrapT = THREE.RepeatWrapping;
        this.wood.map?.dispose();
        this.wood.map = texture;
        this.wood.color.set(0xffffff);
        this.wood.needsUpdate = true;
        this.redrawResting();
      });
    }
  }

  play(
    roll: RollEvent,
    meshes: DieMesh[],
    serverNow: () => number,
    onLand: () => void,
    tags: TagColor[] = [],
  ): boolean {
    if (this.currentId === roll.id && this.playing) return false;
    this.currentId = roll.id;
    this.stop();
    this.currentId = roll.id;
    this.resize();
    this.tagColors = tags;
    this.assets.beginRoll();
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
      const group = mesh ? this.assets.build(mesh, die.tag, this.tagColors) : new THREE.Group();
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
    // Anchor once to server time; later clock corrections cannot jerk a rolling die.
    const initialElapsed = (serverNow() - roll.playAt) / 1000;
    const startedAt = performance.now();
    const loop = () => {
      if (!this.playing || document.hidden) return;
      this.frameId = 0;
      const elapsed = (initialElapsed + (performance.now() - startedAt) / 1000) * ROLL_SPEED;
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
        this.fromRotation.set(from.q[0], from.q[1], from.q[2], from.q[3]);
        this.toRotation.set(to?.q[0] ?? from.q[0], to?.q[1] ?? from.q[1], to?.q[2] ?? from.q[2], to?.q[3] ?? from.q[3]);
        actor.group.quaternion.copy(this.fromRotation).slerp(this.toRotation, frac);
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
        onLand();
      }
      // A hit can remain here until its owner chooses to roll damage. Once the
      // badges have appeared the tray is static, so it needs no further frames.
      if (this.playing && (!landed || elapsed - landedAt < TAG_POP_S)) this.frameId = requestAnimationFrame(loop);
      else this.resume = null;
    };
    this.resume = loop;
    loop();
    return true;
  }

  stop(clear = true) {
    this.playing = false;
    this.currentId = '';
    this.resume = null;
    cancelAnimationFrame(this.frameId);
    this.frameId = 0;
    if (!clear) return;
    const objects = this.actors.flatMap(actor => [actor.group, actor.shadow, ...(actor.badge ? [actor.badge] : [])]);
    this.assets.release(objects);
    for (const object of objects) object.removeFromParent();
    this.actors = [];
  }

  dispose() {
    this.disposed = true;
    document.removeEventListener('visibilitychange', this.visibilityChanged);
    this.stop();
    this.assets.dispose();
    disposeObjects([this.scene]);
    this.renderer.dispose();
  }

  resize() {
    const width = this.canvas.clientWidth || 640;
    const height = this.canvas.clientHeight || 256;
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
    this.redrawResting();
  }

  private redrawResting() {
    if (this.playing && this.frameId === 0) this.renderer.render(this.scene, this.camera);
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
    this.cameraAim.copy(this.trayAim).lerp(this.lineupAim, glide);
    this.camera.lookAt(this.cameraAim);
  }

  private revealFaces() {
    for (const actor of this.actors) this.assets.highlight(actor.group, actor.value);
  }

  private raiseTags() {
    for (const actor of this.actors) {
      actor.badge = valueTag(actor.value, actor.sign, actor.letter, this.tagColors);
      actor.badge.scale.set(0.01, 0.01, 1);
      this.scene.add(actor.badge);
    }
  }

  private placeTags(sinceLand: number) {
    const pop = 1 - (1 - Math.min(1, Math.max(0, sinceLand / TAG_POP_S))) ** 3;
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

function smoothstep(amount: number) {
  const t = Math.min(1, Math.max(0, amount));
  return t * t * (3 - 2 * t);
}
