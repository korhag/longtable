import * as THREE from 'three';
import type { CastAnimations } from '../assets/catalog';
import { AtlasTextureCache } from './atlasTextureCache';
import { CastAtlas } from './castAtlas';
import { cutoutPlacement, opaqueTop } from './cutout';

export const ANIMATION_KINDS = ['idle', 'melee', 'ranged', 'move', 'hurt', 'spell'] as const;
export type CastAnimationKind = typeof ANIMATION_KINDS[number];
export type CastFacing = 'north' | 'east' | 'south' | 'west';
export const CLIP_MS: Record<CastAnimationKind, number> = { idle: 2400, melee: 960, ranged: 1200, move: 1600, hurt: 680, spell: 1800 };
const IDLE_PERIOD = 8000;
const clamp = (v: number) => Math.max(0, Math.min(1, v));
const ease = (v: number) => { const t = clamp(v); return t * t * (3 - 2 * t); };
const pulse = (t: number, a: number, b: number) => t <= a || t >= b ? 0 : Math.sin((t - a) / (b - a) * Math.PI);

/** A retained pose buffer: lean, bend, breath, stride, arms, bob, effect, projectile. */
export function sampleCastPose(kind: CastAnimationKind, progress: number, facing: CastFacing, out: Float32Array, reducedMotion = false) {
  out.fill(0);
  if (reducedMotion || progress <= 0 || progress >= 1) return out;
  const direction = facing === 'west' || facing === 'north' ? -1 : 1;
  const side = facing === 'east' || facing === 'west' ? 1 : .65;
  const t = progress;
  if (kind === 'idle') {
    out[2] = Math.sin(t * Math.PI) * .018;
    out[4] = Math.sin(t * Math.PI * 2) * .009;
  } else if (kind === 'move') {
    const envelope = ease(t / .12) * ease((1 - t) / .12);
    out[0] = direction * .055 * envelope;
    out[3] = Math.sin(t * Math.PI * 8) * .105 * envelope;
    out[4] = -out[3] * .6;
    out[5] = Math.abs(Math.sin(t * Math.PI * 8)) * .025 * envelope;
  } else if (kind === 'melee') {
    const windup = pulse(t, 0, .5), strike = pulse(t, .32, .85);
    out[0] = direction * side * (-.13 * windup + .23 * strike);
    out[1] = -.035 * strike;
    out[4] = direction * (-.16 * windup + .32 * strike);
    out[6] = pulse(t, .36, .72);
  } else if (kind === 'ranged') {
    const aim = ease(t / .25) * (1 - ease((t - .72) / .28));
    const recoil = pulse(t, .52, .8);
    out[0] = direction * side * (.05 * aim - .12 * recoil);
    out[4] = direction * .15 * aim;
    out[1] = -.025 * recoil;
    out[6] = pulse(t, .53, .94);
    out[7] = ease((t - .53) / .41);
  } else if (kind === 'hurt') {
    const flinch = Math.sin(t * Math.PI) * (1 - .4 * t);
    out[0] = -direction * .15 * flinch;
    out[1] = -.09 * flinch;
    out[4] = -direction * .1 * flinch;
    out[6] = pulse(t, 0, .65);
  } else {
    const charge = Math.sin(t * Math.PI);
    out[0] = direction * Math.sin(t * Math.PI * 2) * .04;
    out[2] = charge * .035;
    out[4] = direction * charge * .2;
    out[5] = charge * .025;
    out[6] = charge;
    out[7] = t;
  }
  return out;
}

/** Small CPU rig shared by all four directional pictures. Geometry and VFX are retained. */
export class CastAnimator {
  private rest = new Float32Array();
  private pose = new Float32Array(8);
  private geometry: THREE.BufferGeometry | null = null;
  private width = 1;
  private height = 1;
  private imageHeight = 1;
  private imageTop = 1;
  private facing: CastFacing = 'south';
  private clip: { kind: CastAnimationKind; started: number } | null = null;
  private lastSample = '';
  private deformed = false;
  private restUntil = 0;
  private walkStarted: number | null = null;
  private ring: THREE.Mesh<THREE.RingGeometry, THREE.MeshBasicMaterial>;
  private dart: THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>;
  private atlas: CastAtlas;

  constructor(private card: THREE.Mesh, cache = new AtlasTextureCache()) {
    this.atlas = new CastAtlas(card, cache);
    const effectMaterial = (color: number) => new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0, depthWrite: false, depthTest: true, side: THREE.DoubleSide, blending: THREE.AdditiveBlending });
    this.ring = new THREE.Mesh(new THREE.RingGeometry(.2, .225, 24, 1, 0, Math.PI * 1.5), effectMaterial(0x90d7ff));
    this.dart = new THREE.Mesh(new THREE.PlaneGeometry(.3, .025), effectMaterial(0xffdc87));
    for (const effect of [this.ring, this.dart]) {
      effect.visible = false;
      effect.renderOrder = 3;
      effect.userData.castEffect = true;
      effect.raycast = () => {};
      card.add(effect);
    }
  }

  setFrame(size: [number, number], pivot: [number, number], pixels: number, facing: string, drawn = false) {
    this.atlas.detach();
    const place = cutoutPlacement(size, pivot, pixels);
    const geometry = new THREE.PlaneGeometry(place.width, place.height, drawn ? 1 : 12, drawn ? 1 : 20);
    geometry.translate(place.offsetX, place.offsetY, 0);
    (geometry.getAttribute('position') as THREE.BufferAttribute).setUsage(THREE.DynamicDrawUsage);
    geometry.computeBoundingSphere();
    // Keep broad-phase picking conservative for every deformed pose. Triangle
    // intersections and interpolated UV alpha still use the actual live vertices.
    geometry.boundingSphere!.radius += place.width * .8 + place.height * .2;
    this.card.geometry.dispose();
    this.card.geometry = geometry;
    this.geometry = geometry;
    this.rest = new Float32Array(geometry.getAttribute('position').array);
    this.atlas.setGeometry(geometry, this.rest);
    this.width = place.width;
    this.height = Math.max(.01, place.offsetY + place.height / 2);
    this.imageTop = this.height;
    this.imageHeight = place.height;
    this.facing = (['north', 'east', 'south', 'west'].includes(facing) ? facing : 'south') as CastFacing;
    this.lastSample = '';
    this.deformed = false;
  }

  setImage(image: CanvasImageSource | undefined) {
    const top = opaqueTop(image);
    this.height = Math.max(.01, this.imageTop - (top ?? 0) * this.imageHeight);
    this.lastSample = '';
  }

  setRestTexture(texture: THREE.Texture) {
    this.atlas.setBaseTexture(texture);
    this.setImage(texture.image as CanvasImageSource | undefined);
  }

  setAnimationAtlas(definition: CastAnimations | undefined, url: string | null, filtering: 'linear' | 'nearest', onPaint: () => void) {
    return this.atlas.configure(definition, url, filtering, onPaint);
  }

  clipDuration(kind: CastAnimationKind) { return this.atlas.duration(kind) ?? CLIP_MS[kind]; }

  play(kind: CastAnimationKind, now: number) {
    this.clip = { kind, started: now };
    this.lastSample = '';
  }

  clear() {
    this.clip = null;
    this.walkStarted = null;
    this.lastSample = '';
    this.restore();
  }

  private restore() {
    this.atlas.rest();
    if (this.geometry && this.deformed) {
      const position = this.geometry.getAttribute('position');
      (position.array as Float32Array).set(this.rest);
      position.needsUpdate = true;
    }
    this.deformed = false;
    this.ring.visible = this.dart.visible = false;
  }

  /** Milliseconds until another useful picture; Infinity means no wake-up. */
  tick(now: number, moving: boolean, lowPower: boolean, reducedMotion: boolean, idle = true) {
    if (!this.geometry || !(this.card.material as THREE.MeshBasicMaterial).map || reducedMotion) {
      this.clear(); return Infinity;
    }
    const interval = lowPower ? 80 : 40;
    let kind: CastAnimationKind = 'idle', progress = 0, elapsed = 0, next = interval;
    if (this.clip && now - this.clip.started >= this.clipDuration(this.clip.kind)) {
      this.clip = null; this.restUntil = now + 1200; this.lastSample = ''; this.restore();
    }
    if (this.clip) {
      kind = this.clip.kind;
      elapsed = now - this.clip.started;
      progress = elapsed / this.clipDuration(kind);
    } else if (moving) {
      this.walkStarted ??= now;
      elapsed = now - this.walkStarted;
      kind = 'move'; progress = .12 + (elapsed % 1200) / 1200 * .76;
    } else {
      this.walkStarted = null;
      if (!idle) { this.restore(); return Infinity; }
      if (now < this.restUntil) { this.restore(); return this.restUntil - now; }
      const phase = now % IDLE_PERIOD;
      if (phase >= this.clipDuration('idle')) { this.restore(); return IDLE_PERIOD - phase; }
      elapsed = phase; progress = phase / this.clipDuration('idle');
    }
    if (this.atlas.configured) {
      this.ring.visible = this.dart.visible = false;
      this.deformed = false;
      return this.atlas.show(kind, elapsed, now);
    }
    const sample = `${kind}:${Math.floor(now / interval)}`;
    if (sample === this.lastSample) return next;
    this.lastSample = sample;
    sampleCastPose(kind, progress, this.facing, this.pose);
    this.deform();
    this.effects(kind, this.pose[6]!, this.pose[7]!);
    return next;
  }

  private deform() {
    const position = this.geometry!.getAttribute('position');
    const vertices = position.array as Float32Array;
    const [lean, bend, breath, stride, arms, bob] = this.pose;
    for (let i = 0; i < vertices.length; i += 3) {
      const x = this.rest[i]!, y = this.rest[i + 1]!;
      const h = clamp(y / this.height), anchor = ease(h / .15);
      const side = Math.max(-1, Math.min(1, x / (this.width * .4)));
      const chest = Math.exp(-(((h - .58) / .24) ** 2));
      const lower = Math.exp(-(((h - .19) / .18) ** 2)) * anchor;
      const arm = chest * Math.abs(side) ** 1.6;
      vertices[i] = x + anchor * (lean! * h * this.width + arms! * arm * this.width) + stride! * lower * this.width;
      vertices[i + 1] = y + anchor * (bend! * h * this.height + breath! * chest * this.height + bob! * this.height)
        + Math.max(0, stride! * side) * lower * this.height * .2;
      vertices[i + 2] = this.rest[i + 2]!;
    }
    position.needsUpdate = true;
    this.deformed = true;
  }

  private effects(kind: CastAnimationKind, strength: number, travel: number) {
    const direction = this.facing === 'west' || this.facing === 'north' ? -1 : 1;
    const spell = kind === 'spell', melee = kind === 'melee', hurt = kind === 'hurt';
    this.ring.visible = strength > .01 && (spell || melee || hurt);
    this.dart.visible = strength > .01 && kind === 'ranged';
    if (this.ring.visible) {
      this.ring.material.color.setHex(hurt ? 0xf78773 : spell ? 0x79cfff : 0xffdea3);
      this.ring.material.opacity = strength * (hurt ? .28 : .7);
      this.ring.position.set((hurt ? 0 : direction * .28) * this.width, this.height * (spell ? .73 : .55), .025);
      this.ring.rotation.z = spell ? travel * Math.PI * 4 : direction * strength * Math.PI;
      const scale = this.width * (spell ? .7 + strength : hurt ? 1.4 : 1.8);
      this.ring.scale.set(scale, scale * (melee ? .45 : 1), 1);
    }
    if (this.dart.visible) {
      this.dart.material.opacity = strength * .9;
      this.dart.position.set(direction * (.3 + travel * 1.5) * this.width, this.height * .58 + travel * .08, .03);
      this.dart.rotation.z = direction * .08;
    }
  }

  dispose() {
    this.clear();
    this.atlas.detach();
    for (const effect of [this.ring, this.dart]) {
      effect.removeFromParent(); effect.geometry.dispose(); effect.material.dispose();
    }
  }
}
