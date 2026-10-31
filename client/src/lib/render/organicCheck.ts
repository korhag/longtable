import * as THREE from 'three';
import type { Pack } from '../assets/catalog';
import type { MapLayout } from '../net/bindings/MapLayout.ts';
import { TerrainLayer } from './terrain';

/** Pixel assertions against the production shader: feather organic edges, preserve rigid floors and cliffs. */
export async function checkOrganicTransitions() {
  const picture = (color: string) => {
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 128;
    const context = canvas.getContext('2d')!;
    context.fillStyle = color;
    context.fillRect(0, 0, 128, 128);
    return canvas.toDataURL('image/png');
  };
  const pack: Pack = {
    id: 'blend-check', name: 'Blend check', pixelsPerCell: 128, filtering: 'linear',
    terrains: [], decals: [], walls: [], doors: [], props: [], themes: [], baseUrl: '',
    tiles: ['#800000', '#000080'].map((color, index) => ({
      id: String(index), name: String(index), kind: 'ground', sourceSizePx: [128, 128],
      coverageCells: [1, 1], files: [picture(color)], transitionPolicy: 'organic',
    })),
  };
  const layout: MapLayout = {
    width: 2, height: 1, terrains: ['blend-check/0', 'blend-check/1'], cells: btoa(String.fromCharCode(0, 1)),
    pieces: [], props: [], decals: [], lights: [], seed: 1,
  };
  const renderer = new THREE.WebGLRenderer({ antialias: false });
  renderer.setPixelRatio(1);
  renderer.setSize(256, 128, false);
  const scene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-1, 1, 0.5, -0.5, 0.1, 20);
  camera.position.set(1, 5, 0.5);
  camera.up.set(0, 0, -1);
  camera.lookAt(1, 0, 0.5);
  const terrain = new TerrainLayer(scene);
  const sample = (x: number) => {
    renderer.render(scene, camera);
    const color = new Uint8Array(4);
    const gl = renderer.getContext();
    gl.readPixels(x, 64, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, color);
    return color;
  };
  try {
    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('transition pictures did not load')), 4000);
      let paints = 0;
      terrain.sync(layout, [pack], () => {
        if (++paints === 2) { clearTimeout(timeout); resolve(); }
      });
    });
    const left = sample(64), right = sample(192), seam = sample(128);
    if (left[0]! < 115 || left[2]! > 10 || right[2]! < 115 || right[0]! > 10) throw new Error('organic blending lost tile interiors');
    if (seam[0]! < 20 || seam[2]! < 20) throw new Error(`organic edge was hard: ${[...seam]}`);
    pack.tiles![0]!.transitionPolicy = 'rigid';
    terrain.sync(layout, [pack]);
    const rigid = sample(127);
    if (rigid[0]! < 115 || rigid[2]! > 10) throw new Error('organic texture leaked into rigid floor');
    pack.tiles![0]!.transitionPolicy = 'organic';
    layout.levels = btoa(String.fromCharCode(0, 1));
    terrain.sync(layout, [pack]);
    const cliff = sample(127);
    if (cliff[0]! < 115 || cliff[2]! > 10) throw new Error('organic texture leaked across an elevation edge');
  } finally {
    terrain.clear();
    renderer.dispose();
  }
}
