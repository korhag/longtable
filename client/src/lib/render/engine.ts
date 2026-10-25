// ------------------------------------------------------------
// Module: engine
// Purpose: The tabletop. One textured plane, flat disc tokens,
//          and a fog sheet. It redraws only when the view or
//          the camera changes, so an idle table costs nothing.
// ------------------------------------------------------------

import * as THREE from 'three';

import type { Pack } from '../assets/catalog';
import { findIn } from '../assets/catalog';
import type { DoorView } from '../net/bindings/DoorView.ts';
import type { FogView } from '../net/bindings/FogView.ts';
import type { Role } from '../net/bindings/Role.ts';
import type { SceneView } from '../net/bindings/SceneView.ts';
import type { TokenView } from '../net/bindings/TokenView.ts';
import { bitAt, decodeBits } from '../fog/bits';
import type { Tool } from '../table.svelte';
import { DecalLayer } from './decals';
import { StandeeLayer } from './standees';
import { TerrainLayer } from './terrain';
import { ELEVATION_COS, ELEVATION_SIN, GROUND_DEPTH, HEIGHT_SCALE } from './view';

export interface EngineHooks {
  onMove: (tokenId: string, x: number, y: number) => void;
  onSelect: (tokenId: string | null) => void;
  onBrush: (mode: 'reveal' | 'hide', cells: { x: number; y: number }[]) => void;
  onToggleDoor: (doorId: string, open: boolean) => void;
  onBuild?: (phase: 'down' | 'move' | 'up' | 'hover', point: { x: number; y: number }, button: number) => void;
  canDrag: (token: TokenView) => boolean;
  onMeasure?: (delta: { dx: number; dy: number; x: number; y: number } | null) => void;
}

interface View {
  scene: SceneView | null;
  tokens: TokenView[];
  doors: DoorView[];
  fog: FogView | null;
  role: Role;
  selectedId: string | null;
  tool: Tool;
  packs: Pack[];
  ghost: { x: number; y: number }[];
  buildMode: boolean;
}

interface TokenNode {
  id: string;
  group: THREE.Group;
  disc: THREE.Mesh;
  ring: THREE.Mesh;
  label: THREE.Sprite;
  imageUrl: string | null;
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
  private running = false;
  private view: View = {
    scene: null,
    tokens: [],
    doors: [],
    fog: null,
    role: 'player',
    selectedId: null,
    tool: 'select',
    packs: [],
    ghost: [],
    buildMode: false,
  };
  private mapMesh: THREE.Mesh | null = null;
  private fogMesh: THREE.Mesh | null = null;
  private grid: THREE.LineSegments | null = null;
  private gridStyle: 'off' | 'faint' | 'strong' = 'faint';
  private insets = { left: 0, right: 0, top: 0, bottom: 0 };
  private hoverCell: THREE.LineLoop | null = null;
  private dragLine: THREE.Line | null = null;
  private dragOrigin: { x: number; y: number } | null = null;
  private board: THREE.Mesh | null = null;
  private wallLines: THREE.LineSegments | null = null;
  private doorGroup = new THREE.Group();
  private lightGroup = new THREE.Group();
  private tokenGroup = new THREE.Group();
  private nodes = new Map<string, TokenNode>();
  private raycaster = new THREE.Raycaster();
  private ground = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
  private dragging: { id: string; moved: boolean } | null = null;
  private panning = false;
  private brushing = false;
  private building = false;
  private ghostMesh: THREE.Mesh | null = null;
  private ghostKey = '';
  private terrain: TerrainLayer;
  private decals: DecalLayer;
  private standees: StandeeLayer;
  private lastBrush = 0;
  private pointer = { x: 0, y: 0 };
  private mapKey = '';
  private fogKey = '';
  private framedScene = '';
  private frameId = 0;

  constructor(
    private host: HTMLElement,
    private hooks: EngineHooks,
  ) {
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));
    this.renderer.setClearColor(0x100e0c, 1);
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.host.appendChild(this.renderer.domElement);

    this.scene3 = new THREE.Scene();
    this.scene3.add(new THREE.AmbientLight(0xfff1dc, 0.9));
    const sun = new THREE.DirectionalLight(0xffe2b8, 0.45);
    sun.position.set(8, 18, 4);
    this.scene3.add(sun);
    this.scene3.add(this.doorGroup, this.lightGroup, this.tokenGroup);

    this.camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 400);
    this.terrain = new TerrainLayer(this.scene3);
    this.decals = new DecalLayer(this.scene3);
    this.standees = new StandeeLayer(this.scene3);
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

  start() {
    this.running = true;
    const loop = () => {
      if (!this.running) return;
      this.frameId = requestAnimationFrame(loop);
      if (!this.dirty) return;
      this.renderer.render(this.scene3, this.camera);
      this.dirty = false;
    };
    loop();
  }

  dispose() {
    this.running = false;
    cancelAnimationFrame(this.frameId);
    window.removeEventListener('resize', this.resize);
    this.terrain.clear();
    this.renderer.dispose();
    this.renderer.domElement.remove();
  }

  sync(view: View) {
    this.view = view;
    this.ensureMap();
    this.syncFog();
    this.syncWalls();
    this.syncDoors();
    this.syncLights();
    this.syncTokens();
    this.syncBuilt();
    this.syncGhost();
    this.dirty = true;
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
    this.grid?.geometry.dispose();
    (this.grid?.material as THREE.Material | undefined)?.dispose();
    this.board?.geometry.dispose();
    this.mapMesh?.removeFromParent();
    this.grid?.removeFromParent();
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
        this.dirty = true;
      });
    }

    this.grid = gridLines(width, height);
    this.scene3.add(this.grid);
    this.applyGrid();
    if (this.mapMesh) this.mapMesh.visible = !this.view.scene?.layout;
    const sceneId = this.view.scene?.id ?? 'empty';
    if (sceneId !== this.framedScene) {
      this.framedScene = sceneId;
      this.focus.set(width / 2, 0, height / 2);
      this.fitMap();
    }
  }

  private syncFog() {
    const fog = this.view.fog;
    const { width, height } = this.bounds();
    if (!fog) {
      this.fogMesh?.removeFromParent();
      return;
    }
    const key = `${fog.explored}:${fog.mode}:${JSON.stringify(fog.visiblePolygons)}:${width}`;
    if (key === this.fogKey && this.fogMesh) return;
    this.fogKey = key;
    const canvas = drawFog(fog, width, height);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.needsUpdate = true;
    if (!this.fogMesh) {
      this.fogMesh = new THREE.Mesh(
        plane(width, height),
        new THREE.MeshBasicMaterial({ map: texture, transparent: true, depthWrite: false }),
      );
      this.fogMesh.position.set(0, 0.03, 0);
      this.scene3.add(this.fogMesh);
    } else {
      const material = this.fogMesh.material as THREE.MeshBasicMaterial;
      material.map?.dispose();
      material.map = texture;
      material.needsUpdate = true;
      this.fogMesh.geometry.dispose();
      this.fogMesh.geometry = plane(width, height);
      this.fogMesh.position.set(0, 0.03, 0);
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
    clearGroup(this.lightGroup);
    if (this.view.role !== 'gm') return;
    for (const light of this.view.scene?.lights ?? []) {
      const mesh = new THREE.Mesh(
        new THREE.CircleGeometry(0.18, 16),
        new THREE.MeshBasicMaterial({ color: light.color, transparent: true, opacity: 0.85 }),
      );
      mesh.rotation.x = -Math.PI / 2;
      mesh.position.set(light.x, 0.06, light.y);
      this.lightGroup.add(mesh);
    }
  }

  private syncBuilt() {
    const layout = this.view.scene?.layout ?? null;
    const pack = layout?.terrains[0] ? findIn(this.view.packs, layout.terrains[0], (item) => item.terrains)?.pack : undefined;
    const filtering = pack?.filtering ?? 'linear';
    if (filtering !== this.filtering) {
      this.filtering = filtering;
      this.placeCamera();
    }
    const showAll = this.view.buildMode || this.view.role === 'gm' || this.view.fog?.mode === 'gm';
    this.terrain.sync(layout, this.view.packs);
    this.decals.sync(layout, this.view.packs, showAll ? null : this.view.fog, showAll);
    this.standees.sync(layout, this.view.packs, this.view.doors, showAll ? null : this.view.fog, showAll);
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

  private syncTokens() {
    const seen = new Set<string>();
    for (const token of this.view.tokens) {
      seen.add(token.id);
      let node = this.nodes.get(token.id);
      if (!node) {
        node = createToken(token);
        this.nodes.set(token.id, node);
        this.tokenGroup.add(node.group);
      }
      const dragging = this.dragging?.id === token.id;
      if (!dragging) node.group.position.set(token.x, 0, token.y);
      paintToken(node, token, token.id === this.view.selectedId);
    }
    for (const [id, node] of this.nodes) {
      if (seen.has(id)) continue;
      node.group.removeFromParent();
      this.nodes.delete(id);
    }
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
    const visW = Math.max(1, canvasW - this.insets.left - this.insets.right);
    const visH = Math.max(1, canvasH - this.insets.top - this.insets.bottom);
    const aspect = visW / visH;
    const needH = height * GROUND_DEPTH * 1.25;
    const needW = width * 1.25;
    const full = Math.max(needH, needW / aspect, 8) * (canvasH / visH);
    this.distance = full;
    this.placeCamera();
  }

  centerOn(x: number, y: number) {
    this.focus.set(x, 0, y);
    this.placeCamera();
  }

  zoomBy(factor: number, event?: { clientX: number; clientY: number }) {
    const before = event ? this.hitMap(event) : null;
    this.distance = THREE.MathUtils.clamp(this.distance * factor, 6, 140);
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
    this.dirty = true;
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
    this.distance = this.snap(this.distance);
    const pull = 48;
    this.camera.position.set(this.focus.x, ELEVATION_SIN * pull, this.focus.z + ELEVATION_COS * pull);
    this.camera.up.set(0, HEIGHT_SCALE, -GROUND_DEPTH);
    this.camera.lookAt(this.focus.x, 0, this.focus.z);
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
    this.dirty = true;
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
    this.renderer.setSize(width, height, false);
    this.placeCamera();
  };

  private hitMap(event: { clientX: number; clientY: number }): THREE.Vector3 | null {
    const rect = this.renderer.domElement.getBoundingClientRect();
    const ndc = new THREE.Vector2(
      ((event.clientX - rect.left) / rect.width) * 2 - 1,
      -((event.clientY - rect.top) / rect.height) * 2 + 1,
    );
    this.raycaster.setFromCamera(ndc, this.camera);
    const hit = new THREE.Vector3();
    return this.raycaster.ray.intersectPlane(this.ground, hit) ? hit : null;
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
    return this.view.tokens.find((token) => Math.floor(token.x) === x && Math.floor(token.y) === y)?.id ?? null;
  }

  private onDown = (event: PointerEvent) => {
    this.pointer = { x: event.clientX, y: event.clientY };
    if (event.button === 1 || event.altKey || (event.button === 2 && !this.view.buildMode)) {
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
    if (event.button !== 0) return;
    if (this.view.tool !== 'select' && this.view.role === 'gm') {
      this.brushing = true;
      this.paintBrush(event, true);
      return;
    }
    const tokenId = this.tokenAt(event);
    if (tokenId) {
      const token = this.view.tokens.find((item) => item.id === tokenId);
      this.hooks.onSelect(tokenId);
      if (token && this.hooks.canDrag(token)) {
        this.dragging = { id: tokenId, moved: false };
        this.dragOrigin = { x: token.x, y: token.y };
      }
      return;
    }
    this.hooks.onSelect(null);
    this.panning = true;
  };

  private onMove = (event: PointerEvent) => {
    if (this.panning) {
      this.pan(event);
      return;
    }
    if (this.view.buildMode && this.hooks.onBuild) {
      const hit = this.hitMap(event);
      if (hit) this.hooks.onBuild(this.building ? 'move' : 'hover', { x: hit.x, y: hit.z }, event.button);
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
    const hit = this.hitMap(event);
    if (!hit) return;
    const { width, height } = this.bounds();
    const x = THREE.MathUtils.clamp(Math.floor(hit.x) + 0.5, 0.5, width - 0.5);
    const y = THREE.MathUtils.clamp(Math.floor(hit.z) + 0.5, 0.5, height - 0.5);
    const node = this.nodes.get(this.dragging.id);
    node?.group.position.set(x, 0.02, y);
    this.dragging.moved = true;
    this.showDrag(x, y);
    if (this.dragOrigin) {
      this.hooks.onMeasure?.({
        dx: Math.round(x - this.dragOrigin.x),
        dy: Math.round(y - this.dragOrigin.y),
        x: event.clientX,
        y: event.clientY,
      });
    }
    this.dirty = true;
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
      if (node) this.hooks.onMove(this.dragging.id, node.group.position.x, node.group.position.z);
    } else if (!this.dragging && !this.brushing && this.view.role === 'gm') {
      const travel = Math.hypot(event.clientX - this.pointer.x, event.clientY - this.pointer.y);
      if (travel < 5) this.toggleNearestDoor(event);
    }
    this.dragging = null;
    this.dragOrigin = null;
    if (this.dragLine) this.dragLine.visible = false;
    this.hooks.onMeasure?.(null);
    this.panning = false;
    this.brushing = false;
  };

  private onWheel = (event: WheelEvent) => {
    event.preventDefault();
    this.zoomBy(Math.exp(event.deltaY * 0.0012), event);
  };

  private pan(event: PointerEvent) {
    const rect = this.renderer.domElement.getBoundingClientRect();
    const scale = this.distance / Math.max(1, rect.height);
    this.focus.x -= event.movementX * scale;
    this.focus.z -= (event.movementY * scale) / GROUND_DEPTH;
    this.placeCamera();
  }

  private showCell(x: number, y: number) {
    if (!this.hoverCell) {
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.Float32BufferAttribute([0, 0.03, 0, 1, 0.03, 0, 1, 0.03, 1, 0, 0.03, 1], 3));
      this.hoverCell = new THREE.LineLoop(geometry, new THREE.LineBasicMaterial({ color: 0xf2d48a, transparent: true, opacity: 0.8 }));
      this.scene3.add(this.hoverCell);
    }
    this.hoverCell.position.set(x, 0, y);
    this.hoverCell.visible = true;
    this.dirty = true;
  }

  private hideCell() {
    if (this.hoverCell?.visible) {
      this.hoverCell.visible = false;
      this.dirty = true;
    }
  }

  private showDrag(x: number, y: number) {
    if (!this.dragOrigin) return;
    if (!this.dragLine) {
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(new Float32Array(6), 3));
      this.dragLine = new THREE.Line(geometry, new THREE.LineBasicMaterial({ color: 0xf2d48a }));
      this.scene3.add(this.dragLine);
    }
    const positions = this.dragLine.geometry.getAttribute('position') as THREE.BufferAttribute;
    positions.setXYZ(0, this.dragOrigin.x, 0.04, this.dragOrigin.y);
    positions.setXYZ(1, x, 0.04, y);
    positions.needsUpdate = true;
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
    if (!hit) return;
    let best: { id: string; open: boolean; distance: number } | null = null;
    for (const door of this.view.doors) {
      const distance = distanceToSegment(hit.x, hit.z, door.a, door.b);
      if (distance < 0.45 && (!best || distance < best.distance)) {
        best = { id: door.id, open: door.open, distance };
      }
    }
    if (best) this.hooks.onToggleDoor(best.id, !best.open);
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

function gridLines(width: number, height: number) {
  const points: number[] = [];
  const columns = Math.min(Math.ceil(width), 160);
  const rows = Math.min(Math.ceil(height), 160);
  for (let x = 0; x <= columns; x += 1) points.push(x, 0.012, 0, x, 0.012, height);
  for (let y = 0; y <= rows; y += 1) points.push(0, 0.012, y, width, 0.012, y);
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(points, 3));
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

function createToken(token: TokenView): TokenNode {
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
  const label = new THREE.Sprite(
    new THREE.SpriteMaterial({ map: labelTexture(token.name), transparent: true, depthWrite: false }),
  );
  label.position.set(0, 0.2, 0.62);
  label.scale.set(1.3, 0.34, 1);
  label.userData.tokenId = token.id;
  group.add(disc, ring, label);
  group.position.set(token.x, 0, token.y);
  return { id: token.id, group, disc, ring, label, imageUrl: '' };
}

function paintToken(node: TokenNode, token: TokenView, selected: boolean) {
  const ring = node.ring.material as THREE.MeshBasicMaterial;
  ring.color.set(selected ? 0xf2d48a : token.color);
  node.ring.scale.setScalar(selected ? 1.08 : 1);
  const disc = node.disc.material as THREE.MeshLambertMaterial;
  disc.opacity = token.hidden ? 0.45 : 1;
  disc.transparent = token.hidden;
  if (node.imageUrl !== token.imageUrl) {
    disc.map?.dispose();
    node.imageUrl = token.imageUrl;
    if (token.imageUrl) {
      new THREE.TextureLoader().load(token.imageUrl, (texture) => {
        texture.colorSpace = THREE.SRGBColorSpace;
        disc.map = texture;
        disc.needsUpdate = true;
      });
    } else {
      disc.map = initialTexture(token.name, token.color);
      disc.needsUpdate = true;
    }
  }
  const material = node.label.material as THREE.SpriteMaterial;
  const previous = material.userData.name as string | undefined;
  if (previous !== token.name) {
    material.map?.dispose();
    material.map = labelTexture(token.name);
    material.userData.name = token.name;
    material.needsUpdate = true;
  }
}

function initialTexture(name: string, color: string) {
  const canvas = document.createElement('canvas');
  canvas.width = 256;
  canvas.height = 256;
  const context = canvas.getContext('2d');
  if (!context) return new THREE.CanvasTexture(canvas);
  context.fillStyle = '#1a140f';
  context.beginPath();
  context.arc(128, 128, 120, 0, Math.PI * 2);
  context.fill();
  context.fillStyle = color;
  context.font = '700 96px Cinzel, serif';
  context.textAlign = 'center';
  context.textBaseline = 'middle';
  context.fillText(name.slice(0, 2).toUpperCase(), 128, 136);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
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

function drawFog(fog: FogView, mapWidth: number, mapHeight: number) {
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, fog.width);
  canvas.height = Math.max(1, fog.height);
  const context = canvas.getContext('2d');
  if (!context) return canvas;
  const image = context.createImageData(canvas.width, canvas.height);
  const bits = decodeBits(fog.explored || 'AA==');
  const gm = fog.mode === 'gm';
  const hidden = gm ? 150 : 255;
  const remembered = gm ? 64 : 148;
  const count = canvas.width * canvas.height;
  for (let index = 0; index < count; index += 1) {
    const alpha = bitAt(bits, index) ? remembered : hidden;
    const offset = index * 4;
    image.data[offset] = 6;
    image.data[offset + 1] = 5;
    image.data[offset + 2] = 4;
    image.data[offset + 3] = alpha;
  }
  context.putImageData(image, 0, 0);
  if (!gm && fog.visiblePolygons.length > 0) {
    context.globalCompositeOperation = 'destination-out';
    context.fillStyle = '#000';
    for (const polygon of fog.visiblePolygons) {
      if (polygon.length < 3) continue;
      context.beginPath();
      polygon.forEach(([x, y], index) => {
        const px = (x / mapWidth) * canvas.width;
        const py = (y / mapHeight) * canvas.height;
        if (index === 0) context.moveTo(px, py);
        else context.lineTo(px, py);
      });
      context.closePath();
      context.fill();
    }
  }
  return canvas;
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
