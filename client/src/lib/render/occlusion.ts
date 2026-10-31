// ------------------------------------------------------------
// Module: occlusion
// Purpose: One shared pair of targets for scenery depth and the
//          authorized cast mask. The outline follows the sprite
//          alpha and only shows where scenery is closer to the
//          camera. A stand-in with no picture uses the whole card.
//          The ground depth is kept until the ground or the camera
//          changes, so moving a figure does not redraw it.
//          Hidden cast never enter these targets.
// ------------------------------------------------------------

import * as THREE from 'three';

import { ALPHA_CUTOFF } from './cutout';

const MASK = `
  uniform sampler2D map;
  uniform float hasMap;
  uniform vec3 idColor;
  varying vec2 vUv;
  out vec4 fragColor;
  void main() {
    if (hasMap > 0.5) {
      vec4 tex = texture(map, vUv);
      if (tex.a < ${ALPHA_CUTOFF.toFixed(2)}) discard;
    }
    fragColor = vec4(idColor, 1.0);
  }
`;

const VERT = `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

const COMPOSITE = `
  uniform sampler2D castColor;
  uniform sampler2D castDepth;
  uniform sampler2D sceneryDepth;
  uniform vec3 selectedId;
  out vec4 fragColor;
  void main() {
    ivec2 here = ivec2(gl_FragCoord.xy);
    vec4 id = texelFetch(castColor, here, 0);
    if (id.a < 0.5) discard;
    float castZ = texelFetch(castDepth, here, 0).r;
    float sceneryZ = texelFetch(sceneryDepth, here, 0).r;
    bool edge = false;
    for (int y = -1; y <= 1; y++) {
      for (int x = -1; x <= 1; x++) {
        if (x == 0 && y == 0) continue;
        ivec2 sampleAt = here + ivec2(x, y);
        ivec2 bounds = textureSize(castColor, 0);
        vec4 neighbor = any(lessThan(sampleAt, ivec2(0))) || any(greaterThanEqual(sampleAt, bounds))
          ? vec4(0.0) : texelFetch(castColor, sampleAt, 0);
        if (neighbor.a < 0.5 || neighbor.rgb != id.rgb) edge = true;
      }
    }
    bool occluded = sceneryZ + 0.0004 < castZ;
    if (!(edge && occluded)) discard;
    bool selected = distance(id.rgb, selectedId) < 0.03;
    fragColor = selected ? vec4(0.96, 0.84, 0.48, 0.95) : vec4(0.93, 0.90, 0.84, 0.82);
  }
`;

export class OcclusionOutlines {
  sceneryDraws = 0;
  castDraws = 0;
  private readyStamp = -1;
  private castStamp = -1;
  private castState: { mesh: THREE.Mesh; geometry: THREE.BufferGeometry; positionVersion: number; uvVersion: number; map: THREE.Texture | null; version: number; matrix: THREE.Matrix4 }[] = [];
  private masks = new Map<string, THREE.ShaderMaterial>();
  private scenery: THREE.WebGLRenderTarget | null = null;
  private cast: THREE.WebGLRenderTarget | null = null;
  private scratch = new THREE.Scene();
  private overlay = new THREE.Scene();
  private camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
  private material = new THREE.ShaderMaterial({
    glslVersion: THREE.GLSL3,
    uniforms: {
      castColor: { value: null },
      castDepth: { value: null },
      sceneryDepth: { value: null },
      selectedId: { value: new THREE.Vector3(-1, -1, -1) },
    },
    vertexShader: VERT,
    fragmentShader: COMPOSITE,
    depthTest: false,
    depthWrite: false,
    transparent: true,
  });

  constructor() {
    this.overlay.add(new THREE.Mesh(new THREE.PlaneGeometry(2, 2), this.material));
  }

  draw(
    renderer: THREE.WebGLRenderer,
    scene: THREE.Scene,
    camera: THREE.Camera,
    bodies: THREE.Mesh[],
    selectedId: string | null,
    hideDuringScenery: THREE.Object3D[],
    sceneryToken = 0,
  ) {
    if (bodies.length === 0) {
      for (const material of this.masks.values()) material.dispose();
      this.masks.clear();
      this.castState = [];
      this.castStamp = -1;
      return;
    }
    const size = renderer.getDrawingBufferSize(new THREE.Vector2());
    this.ensure(size.x, size.y);
    const scenery = this.scenery;
    const cast = this.cast;
    if (!scenery || !cast) return;
    if (this.readyStamp !== sceneryToken) {
      const hidden = hideDuringScenery.map((object) => object.visible);
      hideDuringScenery.forEach((object) => {
        object.visible = false;
      });
      renderer.setRenderTarget(scenery);
      renderer.clear();
      renderer.render(scene, camera);
      hideDuringScenery.forEach((object, index) => {
        object.visible = hidden[index] ?? true;
      });
      this.readyStamp = sceneryToken;
      this.sceneryDraws += 1;
    }

    // Torch animation changes color, not silhouettes. Keep the ID/depth mask until
    // a body, its picture, or the camera changes; movement still updates every frame.
    if (this.castChanged(bodies, sceneryToken)) {
      const parked: { mesh: THREE.Mesh; parent: THREE.Object3D | null; material: THREE.Material | THREE.Material[] }[] = [];
      const effects: THREE.Object3D[] = [];
      const wanted = new Set<string>();
      for (const mesh of bodies) {
        const id = mesh.userData.tokenId as string;
        wanted.add(id);
        for (const child of mesh.children) if (child.visible && child.userData.castEffect) { effects.push(child); child.visible = false; }
        parked.push({ mesh, parent: mesh.parent, material: mesh.material });
        this.scratch.add(mesh);
        mesh.matrix.copy(mesh.matrixWorld);
        mesh.matrixAutoUpdate = false;
        mesh.material = maskMaterial(mesh, id, this.masks);
      }
      for (const [id, material] of this.masks) if (!wanted.has(id)) { material.dispose(); this.masks.delete(id); }
      renderer.setRenderTarget(cast);
    // The ID mask must have transparent background. An opaque clear color makes
    // empty pixels look like a far-away character and outlines every visible body.
      const clearColor = renderer.getClearColor(new THREE.Color());
      const clearAlpha = renderer.getClearAlpha();
      renderer.setClearColor(0x000000, 0);
      renderer.clear();
      renderer.render(this.scratch, camera);
      this.castDraws += 1;
      renderer.setClearColor(clearColor, clearAlpha);
      for (const item of parked) {
        item.mesh.material = item.material;
        item.mesh.matrixAutoUpdate = true;
        item.parent?.add(item.mesh);
      }
      for (const effect of effects) effect.visible = true;
    }
    renderer.setRenderTarget(null);

    this.material.uniforms.castColor!.value = cast.texture;
    this.material.uniforms.castDepth!.value = cast.depthTexture;
    this.material.uniforms.sceneryDepth!.value = scenery.depthTexture;
    const selected = selectedId ? idColor(selectedId) : new THREE.Color(-1, -1, -1);
    (this.material.uniforms.selectedId!.value as THREE.Vector3).set(selected.r, selected.g, selected.b);
    const autoClear = renderer.autoClear;
    renderer.autoClear = false;
    renderer.render(this.overlay, this.camera);
    renderer.autoClear = autoClear;
  }

  dispose() {
    this.scenery?.dispose();
    this.cast?.dispose();
    this.material.dispose();
    this.overlay.traverse(object => { if (object instanceof THREE.Mesh) object.geometry.dispose(); });
    for (const material of this.masks.values()) material.dispose();
    this.masks.clear();
    this.castState = [];
    this.scenery = null;
    this.cast = null;
  }

  private ensure(width: number, height: number) {
    if (this.scenery && this.scenery.width === width && this.scenery.height === height) return;
    this.scenery?.dispose();
    this.cast?.dispose();
    this.scenery = target(width, height);
    this.cast = target(width, height);
    this.readyStamp = -1;
    this.castStamp = -1;
  }

  private castChanged(bodies: THREE.Mesh[], sceneryToken: number) {
    let changed = this.castStamp !== sceneryToken || this.castState.length !== bodies.length;
    bodies.forEach((mesh, index) => {
      mesh.updateWorldMatrix(true, false);
      const map = (mesh.material as THREE.MeshBasicMaterial).map ?? null;
      const previous = this.castState[index];
      if (!previous || previous.mesh !== mesh || previous.geometry !== mesh.geometry || previous.map !== map
        || previous.positionVersion !== (mesh.geometry.getAttribute('position') as THREE.BufferAttribute).version
        || previous.uvVersion !== (mesh.geometry.getAttribute('uv') as THREE.BufferAttribute).version
        || previous.version !== (map?.version ?? 0) || !previous.matrix.equals(mesh.matrixWorld)) changed = true;
    });
    if (!changed) return false;
    this.castStamp = sceneryToken;
    this.castState.length = bodies.length;
    bodies.forEach((mesh, index) => {
      const map = (mesh.material as THREE.MeshBasicMaterial).map ?? null;
      const state = this.castState[index];
      if (state) {
        state.mesh = mesh; state.geometry = mesh.geometry; state.map = map; state.version = map?.version ?? 0;
        state.positionVersion = (mesh.geometry.getAttribute('position') as THREE.BufferAttribute).version;
        state.uvVersion = (mesh.geometry.getAttribute('uv') as THREE.BufferAttribute).version;
        state.matrix.copy(mesh.matrixWorld);
      } else this.castState[index] = { mesh, geometry: mesh.geometry, positionVersion: (mesh.geometry.getAttribute('position') as THREE.BufferAttribute).version, uvVersion: (mesh.geometry.getAttribute('uv') as THREE.BufferAttribute).version, map, version: map?.version ?? 0, matrix: mesh.matrixWorld.clone() };
    });
    return true;
  }
}

const solidMap = new THREE.DataTexture(new Uint8Array([255, 255, 255, 255]), 1, 1);
solidMap.needsUpdate = true;

function maskMaterial(mesh: THREE.Mesh, tokenId: string, masks: Map<string, THREE.ShaderMaterial>) {
  const source = mesh.material as THREE.MeshBasicMaterial;
  const map = source.map ?? solidMap;
  const hasMap = source.map ? 1 : 0;
  const key = tokenId;
  const existing = masks.get(key);
  if (existing) {
    existing.uniforms.map!.value = map;
    existing.uniforms.hasMap!.value = hasMap;
    return existing;
  }
  const color = idColor(tokenId);
  const material = new THREE.ShaderMaterial({
    glslVersion: THREE.GLSL3,
    uniforms: {
      map: { value: map },
      hasMap: { value: hasMap },
      idColor: { value: new THREE.Vector3(color.r, color.g, color.b) },
    },
    vertexShader: VERT,
    fragmentShader: MASK,
    depthTest: true,
    depthWrite: true,
    side: THREE.DoubleSide,
  });
  masks.set(key, material);
  return material;
}

function idColor(id: string) {
  let hash = 2166136261;
  for (let index = 0; index < id.length; index += 1) hash = Math.imul(hash ^ id.charCodeAt(index), 16777619);
  const red = ((hash >>> 0) % 180) + 40;
  const green = ((hash >>> 8) % 180) + 40;
  const blue = ((hash >>> 16) % 180) + 40;
  return new THREE.Color(red / 255, green / 255, blue / 255);
}

function target(width: number, height: number) {
  const buffer = new THREE.WebGLRenderTarget(width, height, {
    format: THREE.RGBAFormat,
    type: THREE.UnsignedByteType,
    depthBuffer: true,
  });
  buffer.depthTexture = new THREE.DepthTexture(width, height);
  buffer.depthTexture.format = THREE.DepthFormat;
  buffer.depthTexture.type = THREE.UnsignedIntType;
  return buffer;
}
