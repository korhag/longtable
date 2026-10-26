// ------------------------------------------------------------
// Module: prerendered
// Purpose: One camera-facing picture of an angled map. Depth
//          puts tokens behind walls. Fog is read at the floor
//          cell under each pixel, including a tall wall.
// ------------------------------------------------------------

import * as THREE from 'three';
import { EXRLoader } from 'three/addons/loaders/EXRLoader.js';

import type { DoorView } from '../net/bindings/DoorView.ts';
import type { Presentation } from '../net/bindings/Presentation.ts';
import type { PresentationView } from '../net/bindings/PresentationView.ts';
import { basis } from './projection';

const VERT = `
varying vec2 vUv;
void main() {
  vUv = uv;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
}
`;

const FRAG = `
uniform sampler2D colorMap;
uniform sampler2D depthMap;
uniform sampler2D fogMap;
uniform vec2 mapSize;
uniform float hasDepth;
uniform float hasFog;
uniform float cutout;
uniform vec3 exportTarget;
uniform vec3 exportRight;
uniform vec3 exportUp;
uniform vec3 exportForward;
uniform float exportPull;
uniform vec2 fullSize;
varying vec2 vUv;

void main() {
  vec4 color = texture2D(colorMap, vUv);
  if (cutout > 0.5 && color.a < 0.04) discard;
  float viewZ = texture2D(depthMap, vUv).r;
  vec3 world = exportTarget;
  bool sky = hasDepth > 0.5 && viewZ <= 0.0001;
  if (hasDepth > 0.5 && !sky) {
    float sx = (vUv.x - 0.5) * fullSize.x;
    float sy = (vUv.y - 0.5) * fullSize.y;
    world = exportTarget + exportRight * sx + exportUp * sy + exportForward * (viewZ - exportPull);
    vec4 clip = projectionMatrix * viewMatrix * vec4(world, 1.0);
    float ndc = clip.z / clip.w;
    gl_FragDepth = ndc * 0.5 + 0.5;
  } else if (sky) {
    gl_FragDepth = 1.0;
  }
  if (hasFog > 0.5 && !sky) {
    vec2 fogUv = vec2(world.x / mapSize.x, 1.0 - world.z / mapSize.y);
    vec4 fog = texture2D(fogMap, clamp(fogUv, 0.0, 1.0));
    color.rgb = mix(color.rgb, fog.rgb, fog.a);
  }
  gl_FragColor = vec4(color.rgb, cutout > 0.5 ? color.a : 1.0);
}
`;

interface LayerSync {
  presentation: Presentation;
  viewId: string;
  doors: DoorView[];
  fog: THREE.Texture | null;
  mapWidth: number;
  mapHeight: number;
  maxTextureSize: number;
  onWarning: (message: string) => void;
}

export class PrerenderedLayer {
  readonly group = new THREE.Group();
  private material: THREE.ShaderMaterial;
  private quad: THREE.Mesh;
  private doorGroup = new THREE.Group();
  private doorMaterials: THREE.ShaderMaterial[] = [];
  private cache = new Map<string, THREE.Texture>();
  private order: string[] = [];
  private pinned = new Set<string>();
  private generation = 0;
  private key = '';
  private warned = new Set<string>();
  private placedView = '';
  private placeholder = pixel();
  private colorLoader = new THREE.TextureLoader();
  private depthLoader = new EXRLoader();

  constructor(scene: THREE.Scene) {
    this.material = shaderMaterial(false);
    this.material.uniforms.colorMap.value = this.placeholder;
    this.material.uniforms.depthMap.value = this.placeholder;
    this.material.uniforms.fogMap.value = this.placeholder;
    this.quad = new THREE.Mesh(new THREE.BufferGeometry(), this.material);
    this.quad.renderOrder = 0;
    this.quad.frustumCulled = false;
    this.doorGroup.renderOrder = 1;
    this.group.add(this.quad, this.doorGroup);
    this.group.visible = false;
    scene.add(this.group);
  }

  hide() {
    this.generation += 1;
    this.key = '';
    this.group.visible = false;
    this.clearDoors();
  }

  sync(input: LayerSync) {
    const view = pickView(input.presentation, input.viewId);
    if (!view) {
      this.hide();
      return;
    }
    const lighting = input.presentation.activeLighting;
    const variant = view.variants.find((item) => item.id === lighting) ?? view.variants[0];
    if (!variant) {
      this.hide();
      return;
    }
    if (Math.max(view.imageSize[0], view.imageSize[1]) > input.maxTextureSize) {
      this.warn(input.onWarning, `This picture is larger than the graphics card can show (${input.maxTextureSize} pixels).`);
      this.hide();
      return;
    }
    const doorKey = input.doors.map((door) => `${door.id}:${door.open ? 1 : 0}`).join(',');
    const key = `${view.id}|${variant.id}|${doorKey}`;
    this.group.visible = true;
    this.applyCamera(this.material, view);
    this.material.uniforms.hasFog.value = input.fog ? 1 : 0;
    this.material.uniforms.fogMap.value = input.fog ?? this.placeholder;
    this.material.uniforms.mapSize.value.set(input.mapWidth, input.mapHeight);
    if (this.placedView !== view.id) {
      this.placeQuad(this.quad, view);
      this.placedView = view.id;
    }
    for (const material of this.doorMaterials) {
      material.uniforms.hasFog.value = input.fog ? 1 : 0;
      material.uniforms.fogMap.value = input.fog ?? this.placeholder;
    }
    if (key === this.key) return;
    this.key = key;
    this.generation += 1;
    const generation = this.generation;
    void this.show(input, view, variant.colorUrl, view.depthUrl, lighting, generation);
  }

  dispose() {
    this.generation += 1;
    this.clearDoors();
    this.quad.geometry.dispose();
    this.material.dispose();
    this.placeholder.dispose();
    for (const texture of this.cache.values()) texture.dispose();
    this.cache.clear();
    this.group.removeFromParent();
  }

  private async show(input: LayerSync, view: PresentationView, colorUrl: string, depthUrl: string | null, lighting: string, generation: number) {
    const color = await this.fetch(colorUrl, 'color', generation);
    if (!color || generation !== this.generation) return;
    const depth = depthUrl ? await this.fetch(depthUrl, 'depth', generation) : null;
    if (generation !== this.generation) return;
    this.pinned = new Set([colorUrl, depthUrl ?? ''].filter(Boolean));
    this.material.uniforms.colorMap.value = color;
    this.material.uniforms.depthMap.value = depth ?? this.placeholder;
    this.material.uniforms.hasDepth.value = depth ? 1 : 0;
    this.clearDoors();
    if (input.presentation.capabilities.doors) {
      for (const door of input.presentation.doors) {
        const live = input.doors.find((item) => item.id === door.id);
        if (live?.open !== false) continue;
        const layer = door.layers.find((item) => item.view === view.id && item.lighting === lighting);
        if (!layer) continue;
        const doorColor = await this.fetch(layer.colorUrl, 'color', generation);
        if (!doorColor || generation !== this.generation) return;
        const doorDepth = layer.depthUrl ? await this.fetch(layer.depthUrl, 'depth', generation) : null;
        if (generation !== this.generation) return;
        this.pinned.add(layer.colorUrl);
        if (layer.depthUrl) this.pinned.add(layer.depthUrl);
        const material = shaderMaterial(true);
        this.applyCamera(material, view);
        material.uniforms.colorMap.value = doorColor;
        material.uniforms.depthMap.value = doorDepth ?? this.placeholder;
        material.uniforms.hasDepth.value = doorDepth ? 1 : 0;
        material.uniforms.hasFog.value = input.fog ? 1 : 0;
        material.uniforms.fogMap.value = input.fog ?? this.placeholder;
        material.uniforms.mapSize.value.set(input.mapWidth, input.mapHeight);
        const mesh = new THREE.Mesh(this.quad.geometry.clone(), material);
        mesh.renderOrder = 1;
        mesh.frustumCulled = false;
        this.doorGroup.add(mesh);
        this.doorMaterials.push(material);
      }
    }
    this.evict();
    const order = input.presentation.viewOrder;
    const index = Math.max(0, order.indexOf(view.id));
    const neighbors = [order[(index + 1) % order.length], order[(index - 1 + order.length) % order.length]];
    for (const id of neighbors) {
      if (!id || id === view.id) continue;
      const neighbor = input.presentation.views.find((item) => item.id === id);
      const file = neighbor?.variants.find((item) => item.id === lighting)?.colorUrl;
      if (file) void this.fetch(file, 'color', generation);
    }
  }

  private applyCamera(material: THREE.ShaderMaterial, view: PresentationView) {
    const { forward, up, right } = basis(view.azimuthDeg, view.elevationDeg);
    const uniforms = material.uniforms;
    (uniforms.exportTarget.value as THREE.Vector3).set(view.target[0], view.target[1], view.target[2]);
    (uniforms.exportRight.value as THREE.Vector3).set(right.x, right.y, right.z);
    (uniforms.exportUp.value as THREE.Vector3).set(up.x, up.y, up.z);
    (uniforms.exportForward.value as THREE.Vector3).set(forward.x, forward.y, forward.z);
    uniforms.exportPull.value = view.pull;
    (uniforms.fullSize.value as THREE.Vector2).set(view.orthoHalfWidth * 2, view.orthoHalfHeight * 2);
  }

  private placeQuad(mesh: THREE.Mesh, view: PresentationView) {
    const { up, right } = basis(view.azimuthDeg, view.elevationDeg);
    const center = new THREE.Vector3(view.target[0], view.target[1], view.target[2]);
    const halfW = view.orthoHalfWidth;
    const halfH = view.orthoHalfHeight;
    const rx = right.x;
    const ry = right.y;
    const rz = right.z;
    const ux = up.x;
    const uy = up.y;
    const uz = up.z;
    const corner = (sx: number, sy: number) => [
      center.x + rx * sx + ux * sy,
      center.y + ry * sx + uy * sy,
      center.z + rz * sx + uz * sy,
    ];
    const tl = corner(-halfW, halfH);
    const tr = corner(halfW, halfH);
    const br = corner(halfW, -halfH);
    const bl = corner(-halfW, -halfH);
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute([...tl, ...tr, ...br, ...bl], 3));
    geometry.setAttribute('uv', new THREE.Float32BufferAttribute([0, 1, 1, 1, 1, 0, 0, 0], 2));
    geometry.setIndex([0, 2, 1, 0, 3, 2]);
    mesh.geometry.dispose();
    mesh.geometry = geometry;
  }

  private async fetch(url: string, kind: 'color' | 'depth', generation: number) {
    const cached = this.cache.get(url);
    if (cached) return cached;
    try {
      const texture = kind === 'depth' ? await this.depthLoader.loadAsync(url) : await this.colorLoader.loadAsync(url);
      if (generation !== this.generation) {
        texture.dispose();
        return null;
      }
      if (kind === 'color') {
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.flipY = true;
        texture.anisotropy = 4;
      } else {
        texture.colorSpace = THREE.NoColorSpace;
        texture.flipY = false;
        texture.minFilter = THREE.NearestFilter;
        texture.magFilter = THREE.NearestFilter;
        texture.generateMipmaps = false;
      }
      texture.needsUpdate = true;
      this.cache.set(url, texture);
      this.order.push(url);
      this.evict();
      return texture;
    } catch {
      return null;
    }
  }

  private evict() {
    while (this.order.length > 8) {
      const oldest = this.order.shift();
      if (!oldest) return;
      if (this.pinned.has(oldest)) {
        this.order.push(oldest);
        return;
      }
      this.cache.get(oldest)?.dispose();
      this.cache.delete(oldest);
    }
  }

  private clearDoors() {
    for (const material of this.doorMaterials) material.dispose();
    this.doorMaterials = [];
    for (const child of [...this.doorGroup.children]) {
      const mesh = child as THREE.Mesh;
      mesh.geometry.dispose();
      child.removeFromParent();
    }
  }

  private warn(onWarning: (message: string) => void, message: string) {
    if (this.warned.has(message)) return;
    this.warned.add(message);
    onWarning(message);
  }
}

function pickView(presentation: Presentation, viewId: string) {
  return presentation.views.find((view) => view.id === viewId)
    ?? presentation.views.find((view) => view.id === presentation.defaultView)
    ?? presentation.views[0]
    ?? null;
}

function shaderMaterial(cutout: boolean) {
  return new THREE.ShaderMaterial({
    uniforms: {
      colorMap: { value: null },
      depthMap: { value: null },
      fogMap: { value: null },
      mapSize: { value: new THREE.Vector2(1, 1) },
      hasDepth: { value: 0 },
      hasFog: { value: 0 },
      cutout: { value: cutout ? 1 : 0 },
      exportTarget: { value: new THREE.Vector3() },
      exportRight: { value: new THREE.Vector3(1, 0, 0) },
      exportUp: { value: new THREE.Vector3(0, 1, 0) },
      exportForward: { value: new THREE.Vector3(0, 0, -1) },
      exportPull: { value: 1 },
      fullSize: { value: new THREE.Vector2(1, 1) },
    },
    vertexShader: VERT,
    fragmentShader: FRAG,
    transparent: cutout,
    depthTest: true,
    depthWrite: true,
    side: THREE.DoubleSide,
  });
}

function pixel() {
  const texture = new THREE.DataTexture(new Uint8Array([255, 255, 255, 255]), 1, 1);
  texture.needsUpdate = true;
  return texture;
}
