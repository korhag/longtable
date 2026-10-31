import * as THREE from 'three';
import type { MapLayout } from '../net/bindings/MapLayout';
import type { FogView } from '../net/bindings/FogView';
import { supportHeight } from './levels';
import { inSight } from './seen';

export const MAX_ACTIVE_LIGHTS = 32;
export type SourceKind = 'steady' | 'torch' | 'magic';
export interface SceneLight { x: number; y: number; range: number; color: string; kind?: SourceKind; intensity?: number }
export interface EnvironmentLight { preset: 'day' | 'dusk' | 'night'; brightness: number }
export interface LightingScene { lights?: SceneLight[]; environment?: EnvironmentLight; layout?: MapLayout | null }

export function ambientColor(environment?: EnvironmentLight): THREE.Color {
  const preset = environment?.preset ?? 'day';
  const color = new THREE.Color(preset === 'night' ? '#a1b9f0' : preset === 'dusk' ? '#ffd0a1' : '#ffffff');
  return color.multiplyScalar(Math.max(0.05, Math.min(1, environment?.brightness ?? 1)));
}

/** Smooth, bounded motion; no random frame-to-frame jumps or rapid flashing. */
export function lightPulse(light: SceneLight, seconds: number, reducedMotion = false) {
  const phase = light.x * 1.73 + light.y * 2.39;
  const wave = reducedMotion ? 0 : light.kind === 'torch'
    ? 0.55 * Math.sin(seconds * 2.1 + phase) + 0.3 * Math.sin(seconds * 3.7 + phase * 0.7) + 0.15 * Math.sin(seconds * 1.3 + phase * 1.8)
    : light.kind === 'magic' ? Math.sin(seconds * 0.8 + phase) : 0;
  const amount = light.kind === 'torch' ? 0.065 : light.kind === 'magic' ? 0.025 : 0;
  return { range: light.range * (1 + wave * amount), intensity: (light.intensity ?? 1) * (1 + wave * amount) };
}

export function closestLights(lights: SceneLight[], focus: { x: number; z: number }): SceneLight[] {
  return lights.map((light, index) => ({ light, index, distance: Math.hypot(light.x - focus.x, light.y - focus.z) - light.range }))
    .sort((a, b) => a.distance - b.distance || a.index - b.index).slice(0, MAX_ACTIVE_LIGHTS).map(item => item.light);
}

export const LIGHTING_FRAGMENT = `
  uniform vec3 sceneAmbient;
  uniform int sourceCount;
  uniform vec4 sourcePositions[${MAX_ACTIVE_LIGHTS}];
  uniform vec4 sourceColors[${MAX_ACTIVE_LIGHTS}];
  varying vec3 vSceneLightPosition;
  vec3 sceneIllumination(vec3 position) {
    vec3 light = sceneAmbient;
    for (int i = 0; i < ${MAX_ACTIVE_LIGHTS}; i++) {
      if (i >= sourceCount) break;
      vec3 delta = position - sourcePositions[i].xyz;
      delta.y *= 0.65;
      float falloff = max(0.0, 1.0 - length(delta) / max(sourcePositions[i].w, 0.01));
      falloff *= falloff;
      light += sourceColors[i].rgb * sourceColors[i].a * falloff * 1.75;
    }
    return min(light, vec3(3.0));
  }
`;

/** One instance owns its uniforms. Multiple canvases never share light state. */
export class SceneLighting {
  readonly uniforms = {
    sceneAmbient: { value: new THREE.Color(1, 1, 1) },
    sourceCount: { value: 0 },
    sourcePositions: { value: Array.from({ length: MAX_ACTIVE_LIGHTS }, () => new THREE.Vector4()) },
    sourceColors: { value: Array.from({ length: MAX_ACTIVE_LIGHTS }, () => new THREE.Vector4()) },
  };
  readonly markers = new THREE.Group();
  private materialSet = new WeakSet<THREE.Material>();
  private sources: SceneLight[] = [];
  private layout: MapLayout | null = null;
  private fog: FogView | null = null;
  private showAll = true;
  private selected: { light: SceneLight; height: number; color: THREE.Color; marker: THREE.Sprite | null }[] = [];
  private selectionDirty = true;
  private focusX = NaN;
  private focusZ = NaN;
  private glowTexture: THREE.CanvasTexture | null = null;
  private stateKey = '';
  private motionQuery = typeof matchMedia !== 'undefined' ? matchMedia('(prefers-reduced-motion: reduce)') : null;
  private reducedMotion = this.motionQuery?.matches ?? false;
  private motionChanged: () => void;
  animated = false;

  constructor(scene: THREE.Scene, onMotionChange: () => void = () => {}) {
    scene.add(this.markers);
    this.motionChanged = () => { this.reducedMotion = this.motionQuery?.matches ?? false; onMotionChange(); };
    this.motionQuery?.addEventListener('change', this.motionChanged);
  }

  sync(scene: LightingScene | null, fog: FogView | null, showAll: boolean) {
    const sources = (scene?.lights ?? []).filter(light => Number.isFinite(light.range) && light.range > 0 && (light.intensity ?? 1) > 0);
    this.layout = scene?.layout ?? null;
    this.fog = fog;
    this.showAll = showAll;
    const environment = scene?.environment ?? (scene?.layout as (MapLayout & { environment?: EnvironmentLight }) | null)?.environment;
    this.uniforms.sceneAmbient.value.copy(ambientColor(environment));
    const key = JSON.stringify([environment ?? null, sources, this.layout?.width, this.layout?.height, this.layout?.levels ?? '', this.layout?.stairs, fog?.explored ?? '', showAll]);
    const changed = key !== this.stateKey;
    this.stateKey = key;
    if (changed) { this.sources = sources; this.selectionDirty = true; }
    return changed;
  }

  update(seconds: number, focus: { x: number; z: number }) {
    if (this.selectionDirty || focus.x !== this.focusX || focus.z !== this.focusZ) {
      const nearest = closestLights(this.sources, focus);
      if (this.selectionDirty || nearest.length !== this.selected.length || nearest.some((light, i) => light !== this.selected[i]?.light)) {
        this.selected = nearest.map(light => ({ light, height: supportHeight(this.layout, light.x, light.y) + 0.8, color: new THREE.Color(light.color), marker: null }));
        this.syncMarkers();
      }
      this.focusX = focus.x; this.focusZ = focus.z; this.selectionDirty = false;
    }
    this.animated = !this.reducedMotion && this.selected.some(({ light }) => light.kind === 'torch' || light.kind === 'magic');
    this.uniforms.sourceCount.value = this.selected.length;
    for (let i = 0; i < this.selected.length; i++) {
      const { light, height, color, marker } = this.selected[i]!;
      const pulse = lightPulse(light, seconds, this.reducedMotion);
      this.uniforms.sourcePositions.value[i]!.set(light.x, height, light.y, pulse.range);
      this.uniforms.sourceColors.value[i]!.set(color.r, color.g, color.b, pulse.intensity);
      if (marker) {
        const size = 0.35 + Math.min(0.4, pulse.range * 0.045);
        marker.scale.set(size, size * (light.kind === 'torch' ? 1.45 : 1), 1);
      }
    }
  }

  /** Leave fog, outlines, selection affordances and editor guides unlit. */
  bindScene(scene: THREE.Scene, shouldLight: (mesh: THREE.Mesh) => boolean = () => true) {
    scene.traverse(object => {
      if (!(object instanceof THREE.Mesh) || !shouldLight(object)) return;
      for (const material of Array.isArray(object.material) ? object.material : [object.material]) this.bind(material);
    });
  }

  bind(material: THREE.Material) {
    if (this.materialSet.has(material)) return;
    if (material instanceof THREE.ShaderMaterial && material.uniforms.tiles) {
      Object.assign(material.uniforms, this.uniforms);
      material.vertexShader = 'varying vec3 vSceneLightPosition;\n' + material.vertexShader.replace('gl_Position =', 'vSceneLightPosition = (modelMatrix * vec4(position, 1.0)).xyz;\n gl_Position =');
      material.fragmentShader = LIGHTING_FRAGMENT + `\n vec4 sceneLitTexel(vec4 value) { return linearToOutputTexel(vec4(value.rgb * sceneIllumination(vSceneLightPosition), value.a)); }\n` + material.fragmentShader.replaceAll('linearToOutputTexel(', 'sceneLitTexel(');
    } else if (material instanceof THREE.MeshBasicMaterial || material instanceof THREE.MeshLambertMaterial) {
      const previous = material.onBeforeCompile;
      const previousKey = material.customProgramCacheKey();
      material.onBeforeCompile = (shader, renderer) => {
        previous.call(material, shader, renderer);
        Object.assign(shader.uniforms, this.uniforms);
        shader.vertexShader = 'varying vec3 vSceneLightPosition;\n' + shader.vertexShader.replace('#include <project_vertex>', `#include <project_vertex>\nvec4 lightWorld = vec4(transformed, 1.0);\n#ifdef USE_INSTANCING\nlightWorld = instanceMatrix * lightWorld;\n#endif\nvSceneLightPosition = (modelMatrix * lightWorld).xyz;`);
        shader.fragmentShader = LIGHTING_FRAGMENT + shader.fragmentShader.replace('#include <opaque_fragment>', 'outgoingLight *= sceneIllumination(vSceneLightPosition);\n#include <opaque_fragment>');
      };
      material.customProgramCacheKey = () => previousKey + ':scene-lighting-v1';
    } else return;
    this.materialSet.add(material);
    material.needsUpdate = true;
  }

  dispose() {
    this.motionQuery?.removeEventListener('change', this.motionChanged);
    this.clearMarkers();
    this.markers.removeFromParent();
    this.glowTexture?.dispose();
  }

  private clearMarkers() {
    for (const child of [...this.markers.children]) {
      (child as THREE.Sprite).material.dispose();
      child.removeFromParent();
    }
  }

  private syncMarkers() {
    this.clearMarkers();
    const visible = this.selected.filter(({ light }) => this.showAll || inSight(light, this.fog));
    if (!this.glowTexture && visible.length) {
      const canvas = document.createElement('canvas'); canvas.width = canvas.height = 64;
      const context = canvas.getContext('2d')!;
      const glow = context.createRadialGradient(32, 32, 0, 32, 32, 32);
      glow.addColorStop(0, '#ffffff'); glow.addColorStop(0.12, '#fffffff0'); glow.addColorStop(0.4, '#ffffff55'); glow.addColorStop(1, '#ffffff00');
      context.fillStyle = glow; context.fillRect(0, 0, 64, 64);
      this.glowTexture = new THREE.CanvasTexture(canvas);
    }
    for (const source of visible) {
      const sprite = new THREE.Sprite(new THREE.SpriteMaterial({ map: this.glowTexture, color: source.light.color, transparent: true, opacity: 0.85, depthWrite: false, depthTest: true, blending: THREE.AdditiveBlending }));
      sprite.position.set(source.light.x, source.height, source.light.y);
      this.markers.add(sprite);
      source.marker = sprite;
    }
  }
}
