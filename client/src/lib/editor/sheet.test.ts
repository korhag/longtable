import { describe, expect, it } from 'vitest';
import { sheetGrid, sheetManifest, sheetZip, type SheetSettings } from './sheet';

const settings: SheetSettings = { kind: 'ground', cellWidth: 128, cellHeight: 128, margin: 0, gap: 0, packId: 'woodland', name: 'Woodland', names: '', variants: true, footprintWidth: 1, footprintHeight: 1, pixelsPerWorldUnit: 128, pivotX: 64, pivotY: 127, blocksMovement: true, blocksVision: true };

describe('sheet import', () => {
  it('crops exact grids including margins and gutters without cutting adjacent cells', () => {
    const grid = sheetGrid(266, 266, { ...settings, margin: 3, gap: 4 });
    expect(grid.crops).toEqual([{ x: 3, y: 3, width: 128, height: 128 }, { x: 135, y: 3, width: 128, height: 128 }, { x: 3, y: 135, width: 128, height: 128 }, { x: 135, y: 135, width: 128, height: 128 }]);
    expect(() => sheetGrid(257, 256, settings)).toThrow('does not fit');
    expect(() => sheetGrid(256, 256, { ...settings, cellWidth: 0 })).toThrow('whole pixel');
    expect(() => sheetGrid(256, 256, { ...settings, cellWidth: 64 })).toThrow('128 × 128');
  });
  it('groups forest variations into one paintable terrain', () => {
    const pack = sheetManifest(settings, 4);
    expect(pack.tiles).toHaveLength(1);
    expect(pack.tiles![0].files).toEqual(['frames/1.png', 'frames/2.png', 'frames/3.png', 'frames/4.png']);
    expect(pack.tiles![0].transitionPolicy).toBe('organic');
    expect(sheetManifest({ ...settings, variants: false }, 4).tiles).toHaveLength(4);
  });
  it('assigns four directional frames per cast row and preserves names', () => {
    const s = { ...settings, kind: 'cast' as const, cellHeight: 256, names: 'Human\nElf' };
    expect(sheetGrid(512, 512, s).rows).toBe(2);
    const pack = sheetManifest(s, 8);
    expect(pack.cast!.map(actor => actor.name)).toEqual(['Human', 'Elf']);
    expect(pack.cast![1].views.south.file).toBe('frames/7.png');
    expect(() => sheetGrid(384, 512, s)).toThrow('exactly 4 columns');
    expect(() => sheetManifest(s, 7)).toThrow('incomplete');
  });
  it('keeps wall and door axes and open states distinct', () => {
    const door = sheetManifest({ ...settings, kind: 'door' }, 4).objects![0];
    expect(door.placement).toBe('edge');
    expect(door.renderLayers.map(layer => [layer.axis, layer.state])).toEqual([['h', 'closed'], ['h', 'open'], ['v', 'closed'], ['v', 'open']]);
    expect(sheetManifest({ ...settings, kind: 'wall' }, 2).objects![0].renderLayers.map(layer => layer.axis)).toEqual(['h', 'v']);
  });
  it('rejects unsafe IDs, misplaced pivots, ambiguous names and excessive sheets', () => {
    expect(() => sheetManifest({ ...settings, packId: '../outside' }, 1)).toThrow('Pack ID');
    expect(() => sheetManifest({ ...settings, pivotY: 128 }, 1)).toThrow('inside');
    expect(() => sheetManifest({ ...settings, names: 'Oak\nPine' }, 1)).toThrow('exactly 1');
    expect(() => sheetGrid(8192, 8192, settings)).toThrow('16 megapixels');
    expect(() => sheetGrid(4096, 4096, settings)).toThrow('512 frames');
  });
  it('writes ZIP CRCs, central offsets, and payload bytes in a standard stored archive', () => {
    const bytes = new TextEncoder().encode('123456789');
    const zip = sheetZip([{ name: 'pack.json', bytes }, { name: 'frames/1.png', bytes: new Uint8Array([0, 1, 255]) }]);
    const view = new DataView(zip.buffer);
    expect(view.getUint32(0, true)).toBe(0x04034b50);
    expect(view.getUint32(14, true)).toBe(0xcbf43926); // Standard CRC-32 check vector.
    expect(Array.from(zip.slice(39, 48))).toEqual(Array.from(bytes));
    const end = zip.length - 22, central = view.getUint32(end + 16, true);
    expect(view.getUint32(end, true)).toBe(0x06054b50);
    expect(view.getUint16(end + 10, true)).toBe(2);
    expect(view.getUint32(central, true)).toBe(0x02014b50);
    expect(view.getUint32(central + 42, true)).toBe(0);
    expect(view.getUint32(central + 55 + 42, true)).toBe(48);
  });
});
