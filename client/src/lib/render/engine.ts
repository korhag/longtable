// ------------------------------------------------------------
// Module: engine
// Purpose: The tabletop. One fixed camera, upright figures,
//          and a fog sheet. It redraws only when the view or
//          the camera changes, so an idle table costs nothing.
// ------------------------------------------------------------

import * as THREE from 'three';

import { animationFileUrl, fileUrl, findIn, loadTexture, resolveAnimationView, type CastDef, type Pack } from '../assets/catalog';
import type { DoorView } from '../net/bindings/DoorView.ts';
import type { FogView } from '../net/bindings/FogView.ts';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import type { Role } from '../net/bindings/Role.ts';
import type { SceneView } from '../net/bindings/SceneView.ts';
import type { TokenView } from '../net/bindings/TokenView.ts';
import type { Tool } from '../table.svelte';
import { cameraDepth, centeredSpan, framedDistance, pan, sceneBasis } from './cameraProfile';
import { footprintCells, placeAnchor } from './footprint';
import { cellLevel, FOG_LIFT, sceneRise, supportHeight } from './levels';
import { inSight } from './seen';
import { opaqueTexel, orientUpright } from './cutout';
import { DecalLayer } from './decals';
import { OcclusionOutlines } from './occlusion';
import { roomCutaway } from './roomCutaway';
import { SceneLighting } from './lighting';
import { ringPaint, type CastPaint, type RingMark } from './rings';
import { SceneObjects } from './sceneObjects';
import { StandeeLayer } from './standees';
import { blockingSegments, walkRoute, type RouteLayout, type Segment } from './route';
import { motionFacing, motionPosition, walkMotion, type WalkMotion } from './motion';
import { CastAnimator, type CastAnimationKind } from './castAnimation';
import { AtlasTextureCache } from './atlasTextureCache';
import { figurePress, isClick } from './interaction';
import { stairLanding } from './stairLanding';
import { fogStamp, sameFogPicture, sameFogStamp, type FogStamp } from './fogStamp';
import { drawFog } from './fogImage';
import { fogShell, gridPositions, TerrainLayer } from './terrain';

export interface EngineHooks {
  onMove: (tokenId: string, x: number, y: number) => void;
  onSelect: (tokenId: string | null) => void;
  onBrush: (mode: 'reveal' | 'hide', cells: { x: number; y: number }[]) => void;
  onToggleDoor: (doorId: string, open: boolean) => void;
  onBuild?: (phase: 'down' | 'move' | 'up' | 'hover', point: { x: number; y: number }, button: number) => void;
  canDrag: (token: TokenView) => boolean;
  onMeasure?: (delta: { dx: number; dy: number; x: number; y: number; landX: number; landY: number; note: string | null } | null) => void;
  onWarning?: (message: string) => void;
  onTarget?: (tokenId: string | null) => void;
  onMenu?: (tokenId: string | null, x: number, y: number, point?: { x: number; y: number }) => void;
  onPaint?: () => void;
  onPlace?: (point: { x: number; y: number }) => void;
  onCancelPlace?: () => void;
}

export interface View {
  scene: SceneView | null;
  tokens: TokenView[];
  doors: DoorView[];
  fog: FogView | null;
  role: Role;
  /** Whose eyes this picture uses. Their own figure stays even in the dark. */
  seatId?: string | null;
  selectedId: string | null;
  targetId: string | null;
  hoverId: string | null;
  tool: Tool;
  packs: Pack[];
  ghost: { x: number; y: number }[];
  /** Squares the next click will give a new figure. Empty means placement is off. */
  placeSpan: [number, number] | null;
  buildMode: boolean;
  viewId: string;
  cast: Record<string, CastPaint>;
}

interface TokenNode {
  id: string;
  group: THREE.Group;
  disc: THREE.Mesh;
  ring: THREE.Mesh;
  halo: THREE.Mesh;
  ticks: THREE.Group;
  label: THREE.Sprite;
  card: THREE.Mesh;
  bodyKey: string;
  animator: CastAnimator;
}

const EMPTY = { width: 40, height: 30 };

export class TableEngine {
  private renderer: THREE.WebGLRenderer;
  private scene3: THREE.Scene;
  private camera: THREE.OrthographicCamera;
  private focus = new THREE.Vector3(20, 0, 15);
  private distance = 32;
  private filtering: 'linear' | 'nearest' = 'linear';
  private dirty = true;
  private queued = false;
  private lowPower = preferLowPower();
  readonly stats = { draws: 0, scheduled: 0 };
  private running = false;
  private view: View = {
    scene: null,
    tokens: [],
    doors: [],
    fog: null,
    role: 'player',
    selectedId: null,
    targetId: null,
    hoverId: null,
    tool: 'select',
    packs: [],
    ghost: [],
    placeSpan: null,
    buildMode: false,
    viewId: '',
    cast: {},
  };
  private mapMesh: THREE.Mesh | null = null;
  private fogMesh: THREE.Mesh | null = null;
  private grid: THREE.LineSegments | null = null;
  private gridStyle: 'off' | 'faint' | 'strong' = 'faint';
  private insets = { left: 0, right: 0, top: 0, bottom: 0 };
  private legacyNotice = '';
  private hoverCell: THREE.LineLoop | null = null;
  private dragLine: THREE.Line | null = null;
  private dragNote: string | null = null;
  private dragOrigin: { x: number; y: number } | null = null;
  private board: THREE.Mesh | null = null;
  private wallLines: THREE.LineSegments | null = null;
  private doorGroup = new THREE.Group();
  private lighting: SceneLighting;
  private lightTimer: ReturnType<typeof setTimeout> | null = null;
  private animationDelay = Infinity;
  private reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
  private tokenGroup = new THREE.Group();
  private nodes = new Map<string, TokenNode>();
  private atlasTextures = new AtlasTextureCache(12);
  private movements = new Map<string, WalkMotion>();
  private tokenPositions = new Map<string, { x: number; y: number }>();
  private skipMotion = new Set<string>();
  private legacyNavigation: RouteLayout = { width: 0, height: 0 };
  private legacyBlocks: { walls: SceneView['walls'] | undefined; doors: DoorView[]; segments: Segment[] } | null = null;
  private raycaster = new THREE.Raycaster();
  private ground = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
  private dragging: { id: string; moved: boolean } | null = null;
  private acted = false;
  private panning = false;
  private brushing = false;
  private building = false;
  private ghostMesh: THREE.Mesh | null = null;
  private ghostKey = '';
  private placeMesh: THREE.Mesh | null = null;
  private placeKey = '';
  private terrain: TerrainLayer;
  private decals: DecalLayer;
  private standees: StandeeLayer;
  private objects: SceneObjects;
  private outlines = new OcclusionOutlines();
  private lastBrush = 0;
  private pointer = { x: 0, y: 0 };
  private mapKey = '';
  private fogStamp: FogStamp | null = null;
  private framedScene = '';
  private frameId = 0;
  private fogTexture: THREE.CanvasTexture | null = null;
  private fogGeometryKey = '';

  constructor(
    private host: HTMLElement,
    private hooks: EngineHooks,
  ) {
    this.renderer = new THREE.WebGLRenderer({ antialias: !this.lowPower, alpha: false, powerPreference: this.lowPower ? 'low-power' : 'default' });
    this.renderer.setClearColor(0x100e0c, 1);
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.localClippingEnabled = true;
    this.host.appendChild(this.renderer.domElement);

    this.scene3 = new THREE.Scene();
    this.scene3.add(new THREE.AmbientLight(0xfff1dc, 0.9));
    const sun = new THREE.DirectionalLight(0xffe2b8, 0.45);
    sun.position.set(8, 18, 4);
    this.scene3.add(sun);
    this.scene3.add(this.doorGroup, this.tokenGroup);
    this.lighting = new SceneLighting(this.scene3, () => this.invalidate());

    this.camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 400);
    this.terrain = new TerrainLayer(this.scene3);
    this.decals = new DecalLayer(this.scene3);
    this.standees = new StandeeLayer(this.scene3);
    this.objects = new SceneObjects(this.scene3);
    this.resize();

    const canvas = this.renderer.domElement;
    canvas.addEventListener('pointerdown', this.onDown);
    canvas.addEventListener('pointermove', this.onMove);
    canvas.addEventListener('pointerup', this.onUp);
    canvas.addEventListener('pointerleave', this.onUp);
    canvas.addEventListener('wheel', this.onWheel, { passive: false });
    canvas.addEventListener('contextmenu', (event) => event.preventDefault());
    window.addEventListener('resize', this.resize);
  }

  sample() {
    const size = this.renderer.getDrawingBufferSize(new THREE.Vector2());
    const gl = this.renderer.getContext();
    const info = gl.getExtension('WEBGL_debug_renderer_info');
    return {
      drawingBuffer: [size.x, size.y] as [number, number],
      textures: this.renderer.info.memory.textures,
      geometries: this.renderer.info.memory.geometries,
      draws: this.stats.draws,
      calls: this.lastCalls,
      sceneryPasses: this.outlines.sceneryDraws,
      castPasses: this.outlines.castDraws,
      renderer: info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : 'unavailable',
    };
  }

  start() {
    this.running = true;
    document.addEventListener('visibilitychange', this.onVisible);
    this.reducedMotion.addEventListener('change', this.onMotionPreference);
    this.invalidate();
  }

  private onVisible = () => {
    if (document.hidden) {
      if (this.lightTimer !== null) clearTimeout(this.lightTimer);
      this.lightTimer = null;
      cancelAnimationFrame(this.frameId);
      this.frameId = 0;
      this.queued = false;
    } else this.invalidate();
  };

  private onMotionPreference = () => { for (const node of this.nodes.values()) node.animator.clear(); this.invalidate(); };

  invalidate() {
    this.dirty = true;
    this.requestFrame();
  }

  private requestFrame() {
    if (!this.running || document.hidden || this.queued || !this.dirty) return;
    this.queued = true;
    this.stats.scheduled += 1;
    this.frameId = requestAnimationFrame(() => {
      this.queued = false;
      this.frameId = 0;
      if (!this.running || document.hidden || !this.dirty) return;
      this.dirty = false;
      this.draw();
      if (this.movements.size > 0) {
        if (this.lightTimer !== null) clearTimeout(this.lightTimer);
        this.lightTimer = null;
        this.invalidate();
      } else {
        if (this.lightTimer !== null) clearTimeout(this.lightTimer);
        this.lightTimer = null;
        const delay = Math.min(this.animationDelay, this.lighting.animated ? (this.lowPower ? 66 : 33) : Infinity);
        if (Number.isFinite(delay)) this.lightTimer = setTimeout(() => { this.lightTimer = null; this.invalidate(); }, Math.max(16, delay));
      }
    });
  }

  private lastCalls = 0;
  private sceneryToken = 1;

  get sceneryDraws() {
    return this.outlines.sceneryDraws;
  }

  private bumpScenery() {
    this.sceneryToken += 1;
    this.invalidate();
  }

  renderNow() {
    this.draw();
  }

  private draw() {
    const now = performance.now();
    this.advanceMovement(now);
    this.animationDelay = Infinity;
    for (const [id, node] of this.nodes) this.animationDelay = Math.min(this.animationDelay,
      node.animator.tick(now, this.movements.has(id), this.lowPower, this.reducedMotion.matches, !this.view.buildMode));
    this.stats.draws += 1;
    this.lighting.update(Date.now() / 1000, this.focus);
    try {
      const bodies = [...this.nodes.values()].map((node) => node.card).filter((body) => body.visible);
      this.renderer.render(this.scene3, this.camera);
      this.lastCalls = this.renderer.info.render.calls;
      this.outlines.draw(this.renderer, this.scene3, this.camera, bodies, this.view.selectedId, [this.tokenGroup], this.sceneryToken);
      this.hooks.onPaint?.();
    } catch (error) {
      console.error(error);
    }
  }

  dispose() {
    this.showPlace(null);
    this.running = false;
    if (this.lightTimer !== null) clearTimeout(this.lightTimer);
    this.lighting.dispose();
    cancelAnimationFrame(this.frameId);
    document.removeEventListener('visibilitychange', this.onVisible);
    this.reducedMotion.removeEventListener('change', this.onMotionPreference);
    window.removeEventListener('resize', this.resize);
    this.terrain.clear();
    this.decals.clear();
    this.objects.clear();
    this.standees.clear?.();
    this.outlines.dispose();
    this.fogTexture?.dispose();
    this.fogMesh?.geometry.dispose();
    (this.fogMesh?.material as THREE.Material | undefined)?.dispose();
    for (const node of this.nodes.values()) disposeToken(node);
    this.atlasTextures.dispose();
    this.nodes.clear();
    this.renderer.dispose();
    this.renderer.domElement.remove();
  }

  sync(view: View) {
    if (view.scene?.id !== this.view.scene?.id) {
      this.movements.clear(); this.tokenPositions.clear();
      for (const node of this.nodes.values()) node.animator.clear();
    }
    const changed = !sameView(this.view, view);
    if (!sameScenery(this.view, view)) this.sceneryToken += 1;
    const wasPlacing = this.view.placeSpan != null;
    this.view = view;
    if (wasPlacing && !view.placeSpan && !this.dragging) this.hooks.onMeasure?.(null);
    this.ensureMap();
    this.syncGrid();
    this.syncFog();
    this.syncWalls();
    this.syncDoors();
    const lightingChanged = this.syncLights();
    this.syncTokens();
    this.syncBuilt();
    this.syncGhost();
    if (!view.placeSpan) this.showPlace(null);
    this.applyPresentation();
    this.lighting.bindScene(this.scene3, mesh => {
      if (mesh === this.fogMesh || mesh === this.ghostMesh || mesh === this.placeMesh) return false;
      let parent: THREE.Object3D | null = mesh;
      while (parent) {
        if (parent === this.lighting.markers) return false;
        if (parent === this.tokenGroup) return Boolean(mesh.userData.body);
        parent = parent.parent;
      }
      return true;
    });
    if (changed || lightingChanged) this.invalidate();
  }

  private bounds() {
    return {
      width: this.view.scene?.width ?? EMPTY.width,
      height: this.view.scene?.height ?? EMPTY.height,
    };
  }

  private ensureMap() {
    const { width, height } = this.bounds();
    const image = this.view.scene?.imageUrl ?? '';
    const key = `${width}x${height}:${image}`;
    if (key === this.mapKey && this.mapMesh) return;
    this.mapKey = key;
    this.mapMesh?.geometry.dispose();
    (this.mapMesh?.material as THREE.Material | undefined)?.dispose();
    this.board?.geometry.dispose();
    (this.board?.material as THREE.Material | undefined)?.dispose();
    this.mapMesh?.removeFromParent();
    this.board?.removeFromParent();

    this.board = new THREE.Mesh(
      plane(width + 4, height + 4),
      new THREE.MeshLambertMaterial({ color: 0x3a2718 }),
    );
    this.board.position.set(-2, -0.04, -2);
    this.scene3.add(this.board);

    const material = new THREE.MeshLambertMaterial({ color: image ? 0xffffff : 0x2a241c });
    this.mapMesh = new THREE.Mesh(plane(width, height), material);
    this.mapMesh.position.set(0, 0, 0);
    this.scene3.add(this.mapMesh);
    if (image) {
      const loader = new THREE.TextureLoader();
      loader.load(image, (texture) => {
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.anisotropy = 4;
        material.map = texture;
        material.color.set(0xffffff);
        material.needsUpdate = true;
        this.bumpScenery();
      });
    }

    if (this.mapMesh) this.mapMesh.visible = !this.view.scene?.layout;
    const sceneId = this.view.scene?.id ?? 'empty';
    if (sceneId !== this.framedScene) {
      this.framedScene = sceneId;
      this.focus.set(width / 2, 0, height / 2);
      this.fitMap();
    }
  }

  private gridKey = '';

  private syncGrid() {
    const { width, height } = this.bounds();
    const levels = this.view.scene?.layout?.levels ?? '';
    const key = `${width}x${height}:${levels}`;
    if (key === this.gridKey && this.grid) return;
    this.gridKey = key;
    this.grid?.geometry.dispose();
    (this.grid?.material as THREE.Material | undefined)?.dispose();
    this.grid?.removeFromParent();
    this.grid = gridLines(width, height, this.view.scene?.layout ?? null);
    this.scene3.add(this.grid);
    this.applyGrid();
  }

  private syncFog() {
    const fog = this.view.fog;
    const { width, height } = this.bounds();
    if (!fog) {
      this.fogStamp = null;
      this.fogMesh?.removeFromParent();
      return;
    }
    const levels = this.view.scene?.layout?.levels ?? '';
    if (this.fogStamp && this.fogMesh && sameFogStamp(this.fogStamp, fog, width, levels, height)) return;
    this.fogStamp = fogStamp(fog, width, levels, height);
    const oldTexture = this.fogTexture;
    const resized = oldTexture && (oldTexture.image.width !== Math.max(1, fog.width) || oldTexture.image.height !== Math.max(1, fog.height));
    const canvas = drawFog(fog, width, height, oldTexture?.image as HTMLCanvasElement | undefined);
    const texture = !oldTexture || resized ? new THREE.CanvasTexture(canvas) : oldTexture;
    if (resized) oldTexture.dispose();
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.generateMipmaps = false;
    texture.minFilter = THREE.LinearFilter;
    texture.needsUpdate = true;
    this.fogTexture = texture;
    const geometryKey = `${width}:${height}:${levels}`;
    const shell = !this.fogMesh || this.fogGeometryKey !== geometryKey
      ? fogShell(width, height, this.view.scene?.layout ?? null) : null;
    this.fogGeometryKey = geometryKey;
    if (!this.fogMesh) {
      if (!shell) return;
      this.fogMesh = new THREE.Mesh(
        shell.geometry,
        new THREE.MeshBasicMaterial({
          map: texture,
          transparent: true,
          depthWrite: true,
          depthTest: true,
          alphaTest: 0.2,
          side: THREE.DoubleSide,
        }),
      );
      this.fogMesh.position.set(0, shell.lifted ? 0 : FOG_LIFT, 0);
      this.fogMesh.raycast = () => {};
      this.scene3.add(this.fogMesh);
    } else {
      (this.fogMesh.material as THREE.MeshBasicMaterial).map = texture;
      if (shell) {
        this.fogMesh.geometry.dispose();
        this.fogMesh.geometry = shell.geometry;
        this.fogMesh.position.set(0, shell.lifted ? 0 : FOG_LIFT, 0);
      }
      this.fogMesh.raycast = () => {};
      if (!this.fogMesh.parent) this.scene3.add(this.fogMesh);
    }
  }

  private syncWalls() {
    this.wallLines?.geometry.dispose();
    this.wallLines?.removeFromParent();
    this.wallLines = null;
    if (this.view.scene?.layout) return;
    const walls = this.view.role === 'gm' ? (this.view.scene?.walls ?? []) : [];
    if (walls.length === 0) return;
    const points: number[] = [];
    for (const wall of walls) {
      points.push(wall.a[0], 0.05, wall.a[1], wall.b[0], 0.05, wall.b[1]);
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(points, 3));
    this.wallLines = new THREE.LineSegments(
      geometry,
      new THREE.LineBasicMaterial({ color: 0xe0c17a, transparent: true, opacity: 0.55 }),
    );
    this.scene3.add(this.wallLines);
  }

  private syncDoors() {
    clearGroup(this.doorGroup);
    if (this.view.scene?.layout) return;
    for (const door of this.view.doors) {
      const mesh = segmentQuad(door.a, door.b, 0.12, door.open ? 0xc6a15a : 0x8d3d32, 0.05);
      mesh.userData.doorId = door.id;
      this.doorGroup.add(mesh);
    }
  }

  private syncLights() {
    return this.lighting.sync(this.view.scene, this.view.fog, this.view.buildMode || this.view.role === 'gm' || this.view.fog?.mode === 'gm');
  }

  private cutawayKey = '';

  private syncBuilt() {
    const layout = this.view.scene?.layout ?? null;
    const pack = layout?.terrains[0] ? findIn(this.view.packs, layout.terrains[0], (item) => item.terrains)?.pack : undefined;
    const filtering = pack?.filtering ?? 'linear';
    if (filtering !== this.filtering) {
      this.filtering = filtering;
      this.placeCamera();
    }
    const showAll = this.view.buildMode || this.view.role === 'gm' || this.view.fog?.mode === 'gm';
    const candidates = this.visibleTokens().filter((token) => !token.hidden);
    const selected = candidates.find((token) => token.id === this.view.selectedId);
    const lowered = this.view.buildMode ? new Set<string>() : roomCutaway(layout, selected ? [selected] : candidates);
    const cutawayKey = [...lowered].sort().join(';');
    if (cutawayKey !== this.cutawayKey) {
      this.cutawayKey = cutawayKey;
      this.sceneryToken += 1;
    }
    this.terrain.sync(layout, this.view.packs, () => this.bumpScenery());
    this.decals.sync(layout, this.view.packs, showAll ? null : this.view.fog, showAll, () => this.bumpScenery());
    this.standees.sync(layout, this.view.packs, this.view.doors, showAll ? null : this.view.fog, showAll, () => this.bumpScenery(), lowered);
    this.objects.sync(layout, this.view.packs, this.view.doors, showAll ? null : this.view.fog, showAll, () => this.bumpScenery(), lowered);
  }

  private syncGhost() {
    const key = this.view.ghost.map((cell) => `${cell.x},${cell.y}`).join(';');
    if (key === this.ghostKey) return;
    this.ghostKey = key;
    this.ghostMesh?.geometry.dispose();
    (this.ghostMesh?.material as THREE.Material | undefined)?.dispose();
    this.ghostMesh?.removeFromParent();
    this.ghostMesh = null;
    if (this.view.ghost.length === 0) return;
    const positions: number[] = [];
    for (const cell of this.view.ghost) {
      const y = 0.06;
      positions.push(cell.x, y, cell.y, cell.x + 1, y, cell.y, cell.x + 1, y, cell.y + 1);
      positions.push(cell.x, y, cell.y, cell.x + 1, y, cell.y + 1, cell.x, y, cell.y + 1);
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
    this.ghostMesh = new THREE.Mesh(
      geometry,
      new THREE.MeshBasicMaterial({ color: 0xf2d48a, transparent: true, opacity: 0.35, depthWrite: false }),
    );
    this.ghostMesh.renderOrder = 2;
    this.scene3.add(this.ghostMesh);
  }

  private showPlace(point: { x: number; y: number } | null) {
    const span = this.view.placeSpan;
    const { width, height } = this.bounds();
    const cells = point && span ? footprintCells(point.x, point.y, span[0], span[1], width, height) : [];
    const key = cells.map((cell) => `${cell.x},${cell.y}`).join(';');
    if (key === this.placeKey) return;
    this.placeKey = key;
    this.placeMesh?.geometry.dispose();
    (this.placeMesh?.material as THREE.Material | undefined)?.dispose();
    this.placeMesh?.removeFromParent();
    this.placeMesh = null;
    if (cells.length === 0) {
      this.invalidate();
      return;
    }
    const positions: number[] = [];
    for (const cell of cells) {
      const y = cellLevel(this.view.scene?.layout ?? null, cell.x + 0.5, cell.y + 0.5) + 0.08;
      positions.push(cell.x, y, cell.y, cell.x + 1, y, cell.y, cell.x + 1, y, cell.y + 1);
      positions.push(cell.x, y, cell.y, cell.x + 1, y, cell.y + 1, cell.x, y, cell.y + 1);
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
    this.placeMesh = new THREE.Mesh(
      geometry,
      new THREE.MeshBasicMaterial({ color: 0xf2d48a, transparent: true, opacity: 0.45, depthWrite: false, side: THREE.DoubleSide }),
    );
    this.placeMesh.renderOrder = 3;
    this.scene3.add(this.placeMesh);
    this.invalidate();
  }

  private visibleTokens() {
    if (this.view.role === 'gm') return this.view.tokens;
    return this.view.tokens.filter((token) => !token.hidden && inSight(token, this.view.fog, this.view.seatId));
  }

  private syncTokens() {
    const seen = new Set<string>();
    for (const token of this.visibleTokens()) {
      seen.add(token.id);
      let node = this.nodes.get(token.id);
      if (!node) {
        node = createToken(token, this.atlasTextures);
        this.nodes.set(token.id, node);
        this.tokenGroup.add(node.group);
      }
      const dragging = this.dragging?.id === token.id;
      const previous = this.tokenPositions.get(token.id);
      const changed = previous && (previous.x !== token.x || previous.y !== token.y);
      if (changed && !dragging && !this.skipMotion.delete(token.id) && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
        const from = { x: node.group.position.x, y: node.group.position.z };
        const walk = walkRoute(this.movementLayout(), from, token, this.movementSegments(), token.footprint ?? [1, 1]);
        if (!walk.note && walk.points.length > 1) this.movements.set(token.id, walkMotion(walk.points, performance.now()));
        else this.movements.delete(token.id);
      } else if (changed) this.movements.delete(token.id);
      if (!dragging && !this.movements.has(token.id)) node.group.position.set(token.x, supportHeight(this.view.scene?.layout, token.x, token.y), token.y);
      this.tokenPositions.set(token.id, { x: token.x, y: token.y });
      paintToken(node, token, ringMark(token.id, this.view), this.view, () => this.invalidate());
    }
    for (const [id, node] of this.nodes) {
      if (seen.has(id)) continue;
      node.group.removeFromParent();
      disposeToken(node);
      this.nodes.delete(id);
      this.movements.delete(id);
      this.tokenPositions.delete(id);
      this.skipMotion.delete(id);
    }
  }

  private advanceMovement(now: number) {
    for (const [id, motion] of this.movements) {
      const node = this.nodes.get(id);
      if (!node) { this.movements.delete(id); continue; }
      const point = motionPosition(motion, now);
      const token = this.view.tokens.find(token => token.id === id);
      if (token) paintBody(node, { ...token, facing: point.done ? token.facing : motionFacing(motion, now, token.facing) }, this.view.packs, () => this.invalidate());
      node.group.position.set(point.x, supportHeight(this.view.scene?.layout, point.x, point.z), point.z);
      if (point.done) this.movements.delete(id);
    }
  }

  skipNextMovement(tokenId: string) { this.skipMotion.add(tokenId); }

  /** Local visual feedback only; game actions still travel through the rules protocol. */
  playAnimation(tokenId: string, kind: CastAnimationKind) {
    const node = this.nodes.get(tokenId);
    if (!node || this.reducedMotion.matches) return;
    node.animator.play(kind, performance.now());
    this.invalidate();
  }

  setInsets(insets: { left: number; right: number; top: number; bottom: number }) {
    this.insets = insets;
    this.placeCamera();
  }

  fitMap() {
    const { width, height } = this.bounds();
    this.focus.set(width / 2, 0, height / 2);
    const canvas = this.renderer.domElement;
    const canvasW = canvas.clientWidth || 1;
    const canvasH = canvas.clientHeight || 1;
    const span = centeredSpan(width, height, sceneRise(this.view.scene?.layout));
    this.distance = Math.max(framedDistance(span.width, span.height, canvasW, canvasH, this.insets), 8);
    this.placeCamera();
  }

  centerOn(x: number, y: number) {
    this.focus.set(x, 0, y);
    this.placeCamera();
  }

  screenOf(id: string): { x: number; y: number } | null {
    const node = this.nodes.get(id);
    if (!node) return null;
    const point = node.group.position.clone();
    point.y += 0.8;
    point.project(this.camera);
    if (point.z < -1 || point.z > 1) return null;
    const rect = this.renderer.domElement.getBoundingClientRect();
    return {
      x: Math.round(rect.left + (point.x * 0.5 + 0.5) * rect.width),
      y: Math.round(rect.top + (-point.y * 0.5 + 0.5) * rect.height),
    };
  }

  zoomBy(factor: number, event?: { clientX: number; clientY: number }) {
    const before = event ? this.hitMap(event) : null;
    const [min, max] = [6, 140];
    this.distance = THREE.MathUtils.clamp(this.distance * factor, min, max);
    this.placeCamera();
    if (!before || !event) return;
    const after = this.hitMap(event);
    if (!after) return;
    this.focus.x += before.x - after.x;
    this.focus.z += before.z - after.z;
    this.placeCamera();
  }

  setGrid(style: 'off' | 'faint' | 'strong') {
    this.gridStyle = style;
    this.applyGrid();
    this.bumpScenery();
  }

  cycleGrid() {
    const order = ['off', 'faint', 'strong'] as const;
    const next = order[(order.indexOf(this.gridStyle) + 1) % order.length] ?? 'faint';
    this.setGrid(next);
    return next;
  }

  private applyGrid() {
    if (!this.grid) return;
    const material = this.grid.material as THREE.LineBasicMaterial;
    this.grid.visible = this.gridStyle !== 'off';
    material.opacity = this.gridStyle === 'strong' ? 0.55 : 0.16;
  }

  private placeCamera() {
    const { width, height } = this.bounds();
    this.focus.x = THREE.MathUtils.clamp(this.focus.x, 0, width);
    this.focus.z = THREE.MathUtils.clamp(this.focus.z, 0, height);
    this.focus.y = 0;
    this.distance = this.snap(this.distance);
    const { forward, up } = sceneBasis();
    const depth = cameraDepth(width, Math.max(height, 1), 4);
    this.camera.position.set(
      this.focus.x - forward.x * depth.pull,
      this.focus.y - forward.y * depth.pull,
      this.focus.z - forward.z * depth.pull,
    );
    this.camera.up.set(up.x, up.y, up.z);
    this.camera.lookAt(this.focus);
    this.camera.near = depth.near;
    this.camera.far = Math.max(depth.far, 80);
    const canvas = this.renderer.domElement;
    const canvasW = canvas.clientWidth || 1;
    const canvasH = canvas.clientHeight || 1;
    const aspect = canvasW / canvasH;
    const viewH = this.distance;
    const viewW = viewH * aspect;
    const visibleX = (this.insets.left + (canvasW - this.insets.right)) / 2;
    const visibleY = (this.insets.top + (canvasH - this.insets.bottom)) / 2;
    const ndcX = (visibleX / canvasW) * 2 - 1;
    const ndcY = 1 - (visibleY / canvasH) * 2;
    this.camera.left = -(ndcX + 1) * viewW / 2;
    this.camera.right = this.camera.left + viewW;
    this.camera.bottom = -(ndcY + 1) * viewH / 2;
    this.camera.top = this.camera.bottom + viewH;
    this.camera.updateProjectionMatrix();
    this.bumpScenery();
  }

  private snap(distance: number) {
    if (this.filtering !== 'nearest') return distance;
    const height = this.renderer.domElement.clientHeight || 1;
    const pixels = Math.max(1, Math.round(height / distance));
    return height / pixels;
  }

  private resize = () => {
    const width = this.host.clientWidth || 1;
    const height = this.host.clientHeight || 1;
    const cap = this.lowPower ? 1 : Math.min(window.devicePixelRatio || 1, 1.5);
    const longest = Math.max(width, height, 1);
    const limit = this.lowPower ? 1920 : 2560;
    this.renderer.setPixelRatio(Math.min(cap, limit / longest));
    this.renderer.setSize(width, height, false);
    this.placeCamera();
  };

  private applyPresentation() {
    const scene = this.view.scene;
    const presentation = scene?.presentation ?? null;
    if (this.mapMesh) this.mapMesh.visible = !scene?.layout;
    if (this.board) this.board.visible = true;
    if (this.fogMesh) this.fogMesh.visible = true;
    if (!presentation || !scene) {
      this.legacyNotice = '';
      return;
    }
    if (this.legacyNotice === scene.id) return;
    this.legacyNotice = scene.id;
    this.hooks.onWarning?.('This scene still has an angled picture. The table keeps that picture filed away and draws the fixed view.');
  }

  private hitMap(event: { clientX: number; clientY: number }): THREE.Vector3 | null {
    const rect = this.renderer.domElement.getBoundingClientRect();
    const ndc = new THREE.Vector2(
      ((event.clientX - rect.left) / rect.width) * 2 - 1,
      -((event.clientY - rect.top) / rect.height) * 2 + 1,
    );
    this.raycaster.setFromCamera(ndc, this.camera);
    const surface = this.terrain.hit(this.raycaster);
    if (surface) {
      if (surface.stair && !this.view.buildMode) {
        const landing = stairLanding(this.view.scene?.layout, surface.point.x, surface.point.z);
        if (landing) return new THREE.Vector3(landing.x, surface.point.y, landing.y);
      }
      return surface.point;
    }
    const hit = new THREE.Vector3();
    return this.raycaster.ray.intersectPlane(this.ground, hit) ? hit : null;
  }

  tokenAtPoint(clientX: number, clientY: number): string | null {
    return this.tokenAt({ clientX, clientY } as PointerEvent);
  }

  projectClient(x: number, y: number, z: number): { x: number; y: number } | null {
    const point = new THREE.Vector3(x, y, z);
    point.project(this.camera);
    if (point.z < -1 || point.z > 1) return null;
    const rect = this.renderer.domElement.getBoundingClientRect();
    return {
      x: rect.left + (point.x * 0.5 + 0.5) * rect.width,
      y: rect.top + (-point.y * 0.5 + 0.5) * rect.height,
    };
  }

  private tokenAt(event: PointerEvent): string | null {
    const rect = this.renderer.domElement.getBoundingClientRect();
    const ndc = new THREE.Vector2(
      ((event.clientX - rect.left) / rect.width) * 2 - 1,
      -((event.clientY - rect.top) / rect.height) * 2 + 1,
    );
    this.raycaster.setFromCamera(ndc, this.camera);
    const hits = this.raycaster.intersectObjects(this.tokenGroup.children, true);
    for (const hit of hits) {
      if (!shown(hit.object)) continue;
      const mesh = hit.object as THREE.Mesh;
      if (hit.uv) {
        const map = (mesh.material as THREE.MeshBasicMaterial).map;
        if (!opaqueTexel(map?.image as CanvasImageSource | undefined, hit.uv.x, hit.uv.y)) continue;
      }
      let object: THREE.Object3D | null = hit.object;
      while (object) {
        if (typeof object.userData.tokenId === 'string') return object.userData.tokenId;
        object = object.parent;
      }
    }
    return this.cellToken(event);
  }

  private cellToken(event: { clientX: number; clientY: number }): string | null {
    const hit = this.hitMap(event);
    if (!hit) return null;
    const x = Math.floor(hit.x);
    const y = Math.floor(hit.z);
    return this.visibleTokens().find((token) => coversCell(token, x, y))?.id ?? null;
  }

  private onDown = (event: PointerEvent) => {
    this.pointer = { x: event.clientX, y: event.clientY };
    this.acted = false;
    if (event.button === 1 || event.altKey) {
      this.panning = true;
      return;
    }
    if (this.view.buildMode && this.hooks.onBuild) {
      const hit = this.hitMap(event);
      if (hit) {
        this.building = true;
        this.hooks.onBuild('down', { x: hit.x, y: hit.z }, event.button);
      }
      return;
    }
    if (this.view.placeSpan) {
      if (event.button === 2) {
        this.hooks.onCancelPlace?.();
        this.hooks.onMeasure?.(null);
        this.acted = true;
        return;
      }
      if (event.button === 0) {
        const hit = this.hitMap(event);
        if (hit) this.hooks.onPlace?.({ x: hit.x, y: hit.z });
        this.acted = true;
      }
      return;
    }
    if (event.button === 2) {
      const tokenId = this.tokenAt(event);
      if (tokenId && tokenId !== this.view.selectedId) this.hooks.onTarget?.(tokenId);
      const hit = this.hitMap(event);
      this.hooks.onMenu?.(tokenId, event.clientX, event.clientY, hit ? { x: hit.x, y: hit.z } : undefined);
      this.acted = true;
      return;
    }
    if (event.button !== 0) return;
    if (this.view.tool !== 'select' && this.view.role === 'gm') {
      this.brushing = true;
      this.paintBrush(event, true);
      return;
    }
    const tokenId = this.tokenAt(event);
    if (!tokenId) {
      this.hooks.onTarget?.(null);
      this.panning = true;
      return;
    }
    const token = this.view.tokens.find((item) => item.id === tokenId);
    this.acted = true;
    const intent = figurePress(this.view.selectedId, this.view.targetId, tokenId, !!token && this.hooks.canDrag(token));
    if (intent === 'select') {
      this.hooks.onSelect(tokenId);
      this.hooks.onTarget?.(null);
    } else if (intent === 'drag' && token) {
      this.movements.delete(token.id);
      this.dragging = { id: tokenId, moved: false };
      this.dragOrigin = { x: token.x, y: token.y };
    } else if (intent === 'target') this.hooks.onTarget?.(tokenId);
  };

  private onMove = (event: PointerEvent) => {
    if (this.panning) {
      if (!isClick(this.pointer, event)) this.pan(event);
      return;
    }
    if (this.view.buildMode && this.hooks.onBuild) {
      const hit = this.hitMap(event);
      if (hit) this.hooks.onBuild(this.building ? 'move' : 'hover', { x: hit.x, y: hit.z }, event.button);
      return;
    }
    if (this.view.placeSpan) {
      const hit = this.hitMap(event);
      this.showPlace(hit ? { x: hit.x, y: hit.z } : null);
      const span = this.view.placeSpan;
      const { width, height } = this.bounds();
      const x = hit ? placeAnchor(hit.x, span[0], width) : null;
      const y = hit ? placeAnchor(hit.z, span[1], height) : null;
      this.hooks.onMeasure?.(x == null || y == null ? null : { dx: 0, dy: 0, x: event.clientX, y: event.clientY, landX: x, landY: y, note: null });
      return;
    }
    if (this.brushing) {
      this.paintBrush(event, false);
      return;
    }
    if (!this.dragging) {
      const hit = this.hitMap(event);
      if (hit && !this.view.buildMode) this.showCell(Math.floor(hit.x), Math.floor(hit.z));
      else this.hideCell();
      return;
    }
    if (!this.dragging.moved && isClick(this.pointer, event)) return;
    const hit = this.hitMap(event);
    if (!hit) return;
    const { width, height } = this.bounds();
    const token = this.view.tokens.find((item) => item.id === this.dragging?.id);
    const span = token?.footprint ?? [1, 1];
    const x = placeAnchor(hit.x, span[0], width);
    const y = placeAnchor(hit.z, span[1], height);
    if (x == null || y == null) return;
    const node = this.nodes.get(this.dragging.id);
    node?.group.position.set(x, supportHeight(this.view.scene?.layout, x, y), y);
    this.dragging.moved = true;
    this.showDrag(x, y, span);
    if (this.dragOrigin) {
      this.hooks.onMeasure?.({
        dx: Math.round(x - this.dragOrigin.x),
        dy: Math.round(y - this.dragOrigin.y),
        x: event.clientX,
        y: event.clientY,
        landX: x,
        landY: y,
        note: this.dragNote,
      });
    }
    this.invalidate();
  };

  private onUp = (event: PointerEvent) => {
    if (this.building && this.hooks.onBuild) {
      const hit = this.hitMap(event);
      if (hit) this.hooks.onBuild('up', { x: hit.x, y: hit.z }, event.button);
      this.building = false;
      return;
    }
    if (this.dragging?.moved) {
      const node = this.nodes.get(this.dragging.id);
      if (node) {
        this.requestMove(this.dragging.id, node.group.position.x, node.group.position.z);
        // The server response starts the route; a rejected move keeps the original position.
        const original = this.view.tokens.find(token => token.id === this.dragging?.id);
        if (original) node.group.position.set(original.x, supportHeight(this.view.scene?.layout, original.x, original.y), original.y);
        this.invalidate();
      }
    } else if (!this.dragging && !this.brushing && !this.acted && event.button === 0 && event.type !== 'pointerleave' && isClick(this.pointer, event)) {
      const usedDoor = this.view.role === 'gm' && this.toggleNearestDoor(event);
      if (!usedDoor) {
        const selected = this.view.tokens.find(token => token.id === this.view.selectedId);
        const hit = this.hitMap(event);
        if (selected && hit && this.hooks.canDrag(selected)) this.requestMove(selected.id, hit.x, hit.z);
      }
    }
    this.dragging = null;
    this.dragOrigin = null;
    this.dragNote = null;
    if (this.dragLine) this.dragLine.visible = false;
    this.hooks.onMeasure?.(null);
    this.panning = false;
    this.brushing = false;
  };

  private movementLayout(): RouteLayout | null {
    const scene = this.view.scene;
    if (!scene) return null;
    if (scene.layout) return scene.layout;
    this.legacyNavigation.width = Math.ceil(scene.width);
    this.legacyNavigation.height = Math.ceil(scene.height);
    return this.legacyNavigation;
  }

  private movementSegments(): Segment[] {
    const scene = this.view.scene;
    if (scene?.layout) return blockingSegments(scene.layout, this.view.doors, this.view.packs);
    if (this.legacyBlocks?.walls === scene?.walls && this.legacyBlocks?.doors === this.view.doors) return this.legacyBlocks.segments;
    const segments = blockingSegments(null, this.view.doors, this.view.packs);
    for (const wall of scene?.walls ?? []) segments.push({ a: [wall.a[0], wall.a[1]], b: [wall.b[0], wall.b[1]], kind: 'wall' });
    this.legacyBlocks = { walls: scene?.walls, doors: this.view.doors, segments };
    return segments;
  }

  moveTo(tokenId: string, x: number, y: number) { this.requestMove(tokenId, x, y); }

  private requestMove(tokenId: string, x: number, y: number) {
    const token = this.view.tokens.find(item => item.id === tokenId);
    if (!token || !this.hooks.canDrag(token)) return;
    this.skipMotion.delete(tokenId);
    const { width, height } = this.bounds();
    const span = token.footprint ?? [1, 1];
    const landX = placeAnchor(x, span[0], width), landY = placeAnchor(y, span[1], height);
    if (landX == null || landY == null) return;
    const route = walkRoute(this.movementLayout(), token, { x: landX, y: landY }, this.movementSegments(), span);
    if (route.note) { this.hooks.onWarning?.(route.note); return; }
    this.hooks.onMove(tokenId, landX, landY);
  }

  private onWheel = (event: WheelEvent) => {
    event.preventDefault();
    this.zoomBy(Math.exp(event.deltaY * 0.0012), event);
  };

  private pan(event: PointerEvent) {
    const rect = this.renderer.domElement.getBoundingClientRect();
    const scale = this.distance / Math.max(1, rect.height);
    const shift = pan(event.movementX, event.movementY, scale);
    this.focus.x += shift.x;
    this.focus.z += shift.z;
    this.placeCamera();
  }

  private showCell(x: number, y: number) {
    if (!this.hoverCell) {
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.Float32BufferAttribute([0, 0.03, 0, 1, 0.03, 0, 1, 0.03, 1, 0, 0.03, 1], 3));
      this.hoverCell = new THREE.LineLoop(geometry, new THREE.LineBasicMaterial({ color: 0xf2d48a, transparent: true, opacity: 0.8, depthWrite: false }));
      this.scene3.add(this.hoverCell);
    }
    this.hoverCell.position.set(x, cellLevel(this.view.scene?.layout, x + 0.5, y + 0.5), y);
    this.hoverCell.visible = true;
    this.invalidate();
  }

  private hideCell() {
    if (this.hoverCell?.visible) {
      this.hoverCell.visible = false;
      this.invalidate();
    }
  }

  private showDrag(x: number, y: number, footprint: [number, number]) {
    if (!this.dragOrigin) return;
    const from = this.dragOrigin;
    let points: { x: number; y: number; z: number }[];
    const walk = walkRoute(this.movementLayout(), from, { x, y }, this.movementSegments(), footprint);
    points = walk.points;
    this.dragNote = walk.note;
    if (points.length < 2) {
      if (this.dragLine) this.dragLine.visible = false;
      return;
    }
    const flat = new Float32Array(points.length * 3);
    points.forEach((item, index) => {
      flat[index * 3] = item.x;
      flat[index * 3 + 1] = item.y;
      flat[index * 3 + 2] = item.z;
    });
    const count = (this.dragLine?.geometry.getAttribute('position') as THREE.BufferAttribute | undefined)?.count;
    if (!this.dragLine || count !== points.length) {
      this.dragLine?.geometry.dispose();
      this.dragLine?.removeFromParent();
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(flat, 3));
      this.dragLine = new THREE.Line(geometry, new THREE.LineBasicMaterial({ color: this.dragNote ? 0xc45c4a : 0xf2d48a, depthWrite: false }));
      this.scene3.add(this.dragLine);
    } else {
      const positions = this.dragLine.geometry.getAttribute('position') as THREE.BufferAttribute;
      (positions.array as Float32Array).set(flat);
      positions.needsUpdate = true;
      (this.dragLine.material as THREE.LineBasicMaterial).color.set(this.dragNote ? 0xc45c4a : 0xf2d48a);
    }
    this.dragLine.visible = true;
  }

  private paintBrush(event: PointerEvent, force: boolean) {
    const fog = this.view.fog;
    const hit = this.hitMap(event);
    if (!fog || !hit) return;
    const now = performance.now();
    if (!force && now - this.lastBrush < 70) return;
    this.lastBrush = now;
    const mode = this.view.tool === 'hide' ? 'hide' : 'reveal';
    this.hooks.onBrush(mode, brushCells(hit.x, hit.z, fog));
  }

  private toggleNearestDoor(event: PointerEvent) {
    const hit = this.hitMap(event);
    const pictured = this.objects.doorAt(this.raycaster);
    const door = this.view.doors.find(item => item.id === pictured);
    if (door) { this.hooks.onToggleDoor(door.id, !door.open); return true; }
    if (!hit) return false;
    let best: { id: string; open: boolean; distance: number } | null = null;
    for (const door of this.view.doors) {
      const distance = distanceToSegment(hit.x, hit.z, door.a, door.b);
      if (distance < 0.45 && (!best || distance < best.distance)) {
        best = { id: door.id, open: door.open, distance };
      }
    }
    if (best) { this.hooks.onToggleDoor(best.id, !best.open); return true; }
    return false;
  }
}

function plane(width: number, height: number) {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    'position',
    new THREE.Float32BufferAttribute([0, 0, 0, width, 0, 0, width, 0, height, 0, 0, height], 3),
  );
  geometry.setAttribute('uv', new THREE.Float32BufferAttribute([0, 1, 1, 1, 1, 0, 0, 0], 2));
  geometry.setIndex([0, 2, 1, 0, 3, 2]);
  geometry.computeVertexNormals();
  return geometry;
}

function gridLines(width: number, height: number, layout: MapLayout | null) {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(gridPositions(width, height, layout), 3));
  return new THREE.LineSegments(
    geometry,
    new THREE.LineBasicMaterial({ color: 0xd4a656, transparent: true, opacity: 0.16 }),
  );
}

function segmentQuad(a: [number, number], b: [number, number], width: number, color: number, y: number) {
  const dx = b[0] - a[0];
  const dz = b[1] - a[1];
  const length = Math.hypot(dx, dz) || 0.01;
  const px = (-dz / length) * (width / 2);
  const pz = (dx / length) * (width / 2);
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    'position',
    new THREE.Float32BufferAttribute(
      [
        a[0] + px, y, a[1] + pz,
        b[0] + px, y, b[1] + pz,
        b[0] - px, y, b[1] - pz,
        a[0] - px, y, a[1] - pz,
      ],
      3,
    ),
  );
  geometry.setIndex([0, 1, 2, 0, 2, 3]);
  geometry.computeVertexNormals();
  return new THREE.Mesh(
    geometry,
    new THREE.MeshBasicMaterial({ color, transparent: true, opacity: 0.9, depthWrite: false, side: THREE.DoubleSide }),
  );
}

function createToken(token: TokenView, atlasTextures: AtlasTextureCache): TokenNode {
  const group = new THREE.Group();
  group.userData.tokenId = token.id;
  const disc = new THREE.Mesh(
    new THREE.CircleGeometry(0.42, 40),
    new THREE.MeshLambertMaterial({ color: 0xffffff }),
  );
  disc.rotation.x = -Math.PI / 2;
  disc.position.y = 0.04;
  disc.userData.tokenId = token.id;
  const ring = new THREE.Mesh(
    new THREE.RingGeometry(0.44, 0.5, 40),
    new THREE.MeshBasicMaterial({ color: token.color, side: THREE.DoubleSide }),
  );
  ring.rotation.x = -Math.PI / 2;
  ring.position.y = 0.045;
  ring.userData.tokenId = token.id;
  const halo = new THREE.Mesh(
    new THREE.RingGeometry(0.52, 0.6, 40),
    new THREE.MeshBasicMaterial({ color: 0xffffff, side: THREE.DoubleSide }),
  );
  halo.rotation.x = -Math.PI / 2;
  halo.position.y = 0.05;
  halo.visible = false;
  halo.userData.tokenId = token.id;
  const ticks = tickRing();
  const label = new THREE.Sprite(
    new THREE.SpriteMaterial({ map: labelTexture(token.name), transparent: true, depthWrite: false }),
  );
  label.position.set(0, 0.2, 0.62);
  label.scale.set(1.3, 0.34, 1);
  label.userData.tokenId = token.id;
  const card = new THREE.Mesh(
    new THREE.PlaneGeometry(0.9, 1.55),
    new THREE.MeshBasicMaterial({ transparent: true, alphaTest: 0.35, depthTest: true, depthWrite: true, side: THREE.DoubleSide }),
  );
  card.position.y = 0;
  card.visible = false;
  card.renderOrder = 2;
  card.userData.tokenId = token.id;
  card.userData.body = true;
  group.add(disc, ring, halo, ticks, label, card);
  group.position.set(token.x, 0, token.y);
  return { id: token.id, group, disc, ring, halo, ticks, label, card, bodyKey: '', animator: new CastAnimator(card, atlasTextures) };
}

function tickRing(): THREE.Group {
  const group = new THREE.Group();
  const material = new THREE.MeshBasicMaterial({ color: 0xe23b32, side: THREE.DoubleSide });
  for (let index = 0; index < 8; index += 1) {
    const tick = new THREE.Mesh(new THREE.BoxGeometry(0.16, 0.02, 0.05), material);
    const angle = (index / 8) * Math.PI * 2;
    tick.position.set(Math.cos(angle) * 0.66, 0.055, Math.sin(angle) * 0.66);
    tick.rotation.y = -angle;
    group.add(tick);
  }
  group.visible = false;
  return group;
}

function ringMark(id: string, view: View): RingMark {
  if (id === view.selectedId) return 'control';
  if (id === view.targetId) return 'target';
  if (id === view.hoverId) return 'hover';
  return null;
}

function paintToken(node: TokenNode, token: TokenView, mark: RingMark, view: View, onPaint: () => void) {
  const cast = view.cast[token.id] ?? { tone: 'other' as const, color: token.color };
  const paint = ringPaint(cast.tone, cast.color, mark, view.role === 'gm');
  const span = Math.max(token.footprint?.[0] ?? 1, token.footprint?.[1] ?? 1);
  const ring = node.ring.material as THREE.MeshBasicMaterial;
  ring.color.set(paint.color);
  node.ring.scale.setScalar(paint.scale * span);
  const halo = node.halo.material as THREE.MeshBasicMaterial;
  node.halo.visible = paint.halo != null;
  if (paint.halo) halo.color.set(paint.halo);
  node.halo.scale.setScalar(Math.max(paint.scale, 1.08) * span);
  node.ticks.visible = paint.ticks;
  node.disc.visible = false;
  node.card.visible = true;
  paintBody(node, token, view.packs, onPaint);
  node.label.position.y = 1.7;
  const material = node.label.material as THREE.SpriteMaterial;
  const previous = material.userData.name as string | undefined;
  if (previous !== token.name) {
    material.map?.dispose();
    material.map = labelTexture(token.name);
    material.userData.name = token.name;
    material.needsUpdate = true;
  }
}

function paintBody(node: TokenNode, token: TokenView, packs: View['packs'], onPaint: () => void) {
  const facing = token.facing || 'south';
  const asset = token.castAsset ?? '';
  const found = asset ? findIn(packs, asset, (pack) => pack.cast ?? []) : null;
  const frame = found ? castFrame(found.def, facing) : null;
  // A snapshot can arrive before the asynchronous catalog. The resolved frame
  // must participate in the key so a placeholder is replaced when packs arrive.
  const key = `${asset}:${facing}:${token.color}:${found?.pack.contentHash ?? ''}:${frame?.file ?? ''}`;
  if (node.bodyKey === key) return;
  node.bodyKey = key;
  const size = frame?.sizePx ?? [64, 128];
  const pivot = frame?.pivotPx ?? [32, 120];
  const pixels = found?.def.pixelsPerWorldUnit ?? 128;
  node.animator.setFrame(size, pivot, pixels, facing, !!found?.def.animations);
  if (found) {
    const sheet = resolveAnimationView(found.def.animations, facing);
    void node.animator.setAnimationAtlas(sheet?.definition, animationFileUrl(found.pack, sheet?.file), found.pack.filtering, onPaint);
  }
  orientUpright(node.card);
  const material = node.card.material as THREE.MeshBasicMaterial;
  material.color.set(token.color);
  material.alphaTest = 0.5;
  material.depthWrite = true;
  material.depthTest = true;
  material.transparent = false;
  const url = found ? fileUrl(found.pack, frame?.file) : null;
  if (!url || !found) {
    material.map = null;
    material.needsUpdate = true;
    return;
  }
  void loadTexture(url, found.pack.filtering).then((texture) => {
    if (!texture || node.bodyKey !== key || !node.group.parent) return;
    node.animator.setRestTexture(texture);
    material.color.set('#ffffff');
    material.needsUpdate = true;
    onPaint();
  });
}

function coversCell(token: TokenView, x: number, y: number) {
  const [width, depth] = token.footprint ?? [1, 1];
  return bodyCells(token.x, width).includes(x) && bodyCells(token.y, depth).includes(y);
}

function bodyCells(center: number, span: number) {
  const size = Math.max(1, span || 1);
  const start = Math.floor(center - size / 2);
  const cells = [];
  for (let step = 0; step < size; step += 1) cells.push(start + step);
  return cells;
}

function castFrame(actor: CastDef, facing: string) {
  if (facing === 'east') return actor.views.east;
  if (facing === 'west') return actor.views.west;
  if (facing === 'north') return actor.views.north;
  return actor.views.south;
}

function disposeToken(node: TokenNode) {
  node.animator.dispose();
  node.group.traverse((child) => {
    if (child instanceof THREE.Mesh) {
      child.geometry.dispose();
      const material = child.material as THREE.Material;
      material.dispose();
    } else if (child instanceof THREE.Sprite) {
      child.material.map?.dispose();
      child.material.dispose();
    }
  });
}

function shown(object: THREE.Object3D) {
  let current: THREE.Object3D | null = object;
  while (current) {
    if (!current.visible) return false;
    current = current.parent;
  }
  return true;
}

function sameView(left: View, right: View) {
  const leftScene = left.scene;
  const rightScene = right.scene;
  return leftScene?.id === rightScene?.id
    && leftScene?.width === rightScene?.width
    && leftScene?.height === rightScene?.height
    && leftScene?.layout?.cells === rightScene?.layout?.cells
    && (leftScene?.layout?.levels ?? '') === (rightScene?.layout?.levels ?? '')
    && sameStairs(leftScene?.layout?.stairs, rightScene?.layout?.stairs)
    && sameTokens(left.tokens, right.tokens)
    && sameDoors(left.doors, right.doors)
    && sameProps(leftScene?.layout?.props, rightScene?.layout?.props)
    && samePieces(leftScene?.layout?.pieces, rightScene?.layout?.pieces)
    && samePacks(left.packs, right.packs)
    && left.selectedId === right.selectedId
    && left.targetId === right.targetId
    && left.role === right.role
    && sameFogPicture(left.fog, right.fog)
    && (left.seatId ?? '') === (right.seatId ?? '')
    && left.buildMode === right.buildMode
    && left.tool === right.tool;
}

function sameScenery(left: View, right: View) {
  const leftScene = left.scene;
  const rightScene = right.scene;
  const leftPlace = left.placeSpan;
  const rightPlace = right.placeSpan;
  return leftScene?.id === rightScene?.id
    && leftScene?.width === rightScene?.width
    && leftScene?.height === rightScene?.height
    && (leftScene?.imageUrl ?? '') === (rightScene?.imageUrl ?? '')
    && leftScene?.layout?.cells === rightScene?.layout?.cells
    && (leftScene?.layout?.levels ?? '') === (rightScene?.layout?.levels ?? '')
    && sameStairs(leftScene?.layout?.stairs, rightScene?.layout?.stairs)
    && sameProps(leftScene?.layout?.props, rightScene?.layout?.props)
    && samePieces(leftScene?.layout?.pieces, rightScene?.layout?.pieces)
    && sameDoors(left.doors, right.doors)
    && samePacks(left.packs, right.packs)
    && sameFogSheet(left.fog, right.fog)
    && sameCells(left.ghost, right.ghost)
    && leftPlace?.[0] === rightPlace?.[0]
    && leftPlace?.[1] === rightPlace?.[1]
    && left.role === right.role
    && left.buildMode === right.buildMode;
}

function sameTokens(left: View['tokens'], right: View['tokens']) {
  if (left === right) return true;
  if (left.length !== right.length) return false;
  return left.every((token, index) => {
    const other = right[index];
    return !!other
      && token.id === other.id
      && token.x === other.x
      && token.y === other.y
      && token.facing === other.facing
      && (token.castAsset ?? '') === (other.castAsset ?? '')
      && token.hidden === other.hidden
      && (token.ownerId ?? '') === (other.ownerId ?? '')
      && token.name === other.name;
  });
}

function sameDoors(left: View['doors'], right: View['doors']) {
  if (left === right) return true;
  if (left.length !== right.length) return false;
  return left.every((door, index) => door.id === right[index]?.id && door.open === right[index]?.open);
}

function sameProps(left: View['scene'] extends infer _ ? NonNullable<View['scene']>['layout'] extends infer layout ? layout extends { props: infer props } ? props : undefined : undefined : undefined, right: typeof left) {
  const a = left ?? [];
  const b = right ?? [];
  if (a === b) return true;
  if (a.length !== b.length) return false;
  return a.every((prop, index) => prop.id === b[index]?.id && prop.x === b[index]?.x && prop.y === b[index]?.y && prop.asset === b[index]?.asset);
}

function samePieces(left: NonNullable<NonNullable<View['scene']>['layout']>['pieces'] | undefined, right: NonNullable<NonNullable<View['scene']>['layout']>['pieces'] | undefined) {
  const a = left ?? [];
  const b = right ?? [];
  if (a === b) return true;
  if (a.length !== b.length) return false;
  return a.every((piece, index) => piece.id === b[index]?.id && piece.asset === b[index]?.asset && piece.open === b[index]?.open && piece.axis === b[index]?.axis);
}

function sameStairs(left: NonNullable<NonNullable<View['scene']>['layout']>['stairs'], right: NonNullable<NonNullable<View['scene']>['layout']>['stairs']) {
  const a = left ?? [];
  const b = right ?? [];
  if (a === b) return true;
  if (a.length !== b.length) return false;
  return a.every((stair, index) => stair.asset === b[index]?.asset && stair.x === b[index]?.x && stair.y === b[index]?.y && stair.x2 === b[index]?.x2 && stair.y2 === b[index]?.y2);
}

function samePacks(left: View['packs'], right: View['packs']) {
  if (left === right) return true;
  if (left.length !== right.length) return false;
  return left.every((pack, index) => pack.id === right[index]?.id && (pack.contentHash ?? '') === (right[index]?.contentHash ?? ''));
}

function sameCells(left: View['ghost'], right: View['ghost']) {
  if (left === right) return true;
  if (left.length !== right.length) return false;
  return left.every((cell, index) => cell.x === right[index]?.x && cell.y === right[index]?.y);
}

function sameFogSheet(left: View['fog'], right: View['fog']) {
  return sameFogPicture(left, right);
}

function preferLowPower() {
  try {
    return globalThis.localStorage?.getItem('longtable-power') !== 'standard';
  } catch {
    return true;
  }
}

function labelTexture(name: string) {
  const canvas = document.createElement('canvas');
  canvas.width = 512;
  canvas.height = 128;
  const context = canvas.getContext('2d');
  if (!context) return new THREE.CanvasTexture(canvas);
  context.clearRect(0, 0, 512, 128);
  context.font = '600 64px Inter, sans-serif';
  context.textAlign = 'center';
  context.textBaseline = 'middle';
  context.lineWidth = 10;
  context.strokeStyle = 'rgba(0,0,0,0.7)';
  context.strokeText(name, 256, 64);
  context.fillStyle = '#f4ecdf';
  context.fillText(name, 256, 64);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function brushCells(x: number, y: number, fog: FogView) {
  const scale = fog.cellsPerSquare || 4;
  const radius = 0.7 * scale;
  const cx = x * scale;
  const cy = y * scale;
  const cells: { x: number; y: number }[] = [];
  for (let iy = Math.floor(cy - radius); iy <= Math.ceil(cy + radius); iy += 1) {
    for (let ix = Math.floor(cx - radius); ix <= Math.ceil(cx + radius); ix += 1) {
      const dx = ix + 0.5 - cx;
      const dy = iy + 0.5 - cy;
      if (dx * dx + dy * dy <= radius * radius) cells.push({ x: ix, y: iy });
    }
  }
  return cells;
}

function distanceToSegment(x: number, y: number, a: [number, number], b: [number, number]) {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const length = dx * dx + dy * dy || 1;
  const t = Math.min(1, Math.max(0, ((x - a[0]) * dx + (y - a[1]) * dy) / length));
  const px = a[0] + dx * t - x;
  const py = a[1] + dy * t - y;
  return Math.hypot(px, py);
}

function clearGroup(group: THREE.Group) {
  for (const child of [...group.children]) {
    child.removeFromParent();
    if (child instanceof THREE.Mesh) {
      child.geometry.dispose();
      (child.material as THREE.Material).dispose();
    }
  }
}
