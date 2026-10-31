import * as THREE from 'three';
import { cachedImage, type Pack } from '../assets/catalog';
import type { MapLayout } from '../net/bindings/MapLayout';
import { TerrainLayer } from './terrain';

/** Four different world cells catch a double Y flip that symmetric maps cannot. */
export async function checkTerrainOrientation() {
  const renderer = new THREE.WebGLRenderer({ antialias: false });
  renderer.setSize(128, 128, false);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  const colors = ['#ed2010', '#16d030', '#2040e8', '#eed020'];
  const files = colors.map(color => {
    const canvas = document.createElement('canvas'); canvas.width = canvas.height = 128;
    const context = canvas.getContext('2d')!; context.fillStyle = color; context.fillRect(0, 0, 128, 128);
    return canvas.toDataURL();
  });
  await Promise.all(files.map(cachedImage));
  const pack = { id: 'orientation', pixelsPerCell: 128, baseUrl: '', filtering: 'nearest', terrains: [], tiles: files.map((file, index) => ({
    id: `cell-${index}`, name: `Cell ${index}`, kind: 'ground', sourceSizePx: [128, 128], coverageCells: [1, 1], files: [file], transitionPolicy: 'rigid',
  })) } as unknown as Pack;
  const layout: MapLayout = { width: 2, height: 2, terrains: colors.map((_, index) => `orientation/cell-${index}`), cells: btoa(String.fromCharCode(0, 1, 2, 3)), pieces: [], props: [], decals: [], lights: [] };
  const scene = new THREE.Scene(), terrain = new TerrainLayer(scene);
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 20);
  camera.position.set(1, 10, 1); camera.up.set(0, 0, -1); camera.lookAt(1, 0, 1);
  try {
    await new Promise<void>((resolve, reject) => {
      let loaded = 0;
      const timeout = setTimeout(() => reject(new Error('terrain orientation textures did not load')), 5000);
      terrain.sync(layout, [pack], () => { if (++loaded === 4) { clearTimeout(timeout); resolve(); } });
    });
    renderer.render(scene, camera);
    const size = renderer.getDrawingBufferSize(new THREE.Vector2());
    const gl = renderer.getContext(), pixel = new Uint8Array(4);
    for (const [index, [x, z]] of [[0.5, 0.5], [1.5, 0.5], [0.5, 1.5], [1.5, 1.5]].entries()) {
      gl.readPixels(Math.floor(x! / 2 * size.x), Math.floor((1 - z! / 2) * size.y), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
      const color = colors[index]!;
      const expected = [1, 3, 5].map(offset => parseInt(color.slice(offset, offset + 2), 16));
      if (expected.some((channel, n) => Math.abs(channel - pixel[n]!) > 5)) throw new Error(`Terrain cell ${x},${z} shows the wrong row: ${[...pixel]} versus ${expected}`);
    }
  } finally { terrain.clear(); renderer.dispose(); }
}
