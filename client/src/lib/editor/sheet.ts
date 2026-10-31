import type { CastDef, ObjectDef, Pack, TileDef } from '../assets/catalog';

export type SheetKind = 'ground' | 'overlay' | 'object' | 'wall' | 'door' | 'cast';
export interface SheetSettings {
  kind: SheetKind; cellWidth: number; cellHeight: number; margin: number; gap: number;
  packId: string; name: string; names: string; variants: boolean;
  footprintWidth: number; footprintHeight: number; pixelsPerWorldUnit: number;
  pivotX: number; pivotY: number; blocksMovement: boolean; blocksVision: boolean;
}
export interface Crop { x: number; y: number; width: number; height: number }
export const framesPerAsset = (kind: SheetKind) => kind === 'cast' || kind === 'door' ? 4 : kind === 'wall' ? 2 : 1;

/** Require exact cells so an unnoticed border cannot shift every frame. */
export function sheetGrid(width: number, height: number, settings: SheetSettings): { columns: number; rows: number; crops: Crop[] } {
  const { cellWidth: cw, cellHeight: ch, margin: m, gap: g, kind } = settings;
  if (![width, height, cw, ch, m, g].every(Number.isInteger) || cw < 1 || ch < 1 || cw > 2048 || ch > 2048 || m < 0 || g < 0) throw new Error('Use whole pixel dimensions; each cell must be 1–2048 pixels.');
  if (width > 8192 || height > 8192 || width * height > 16_777_216) throw new Error('Keep the sheet below 16 megapixels and 8192 pixels per side.');
  if (kind === 'ground' && (cw !== 128 || ch !== 128)) throw new Error('Ground cells must be exactly 128 × 128 pixels.');
  const columns = (width - 2 * m + g) / (cw + g);
  const rows = (height - 2 * m + g) / (ch + g);
  if (!Number.isInteger(columns) || !Number.isInteger(rows) || columns < 1 || rows < 1) throw new Error('The grid does not fit the sheet exactly. Check cell size, outer margin, and gutter.');
  if (columns * rows > 512) throw new Error('Import at most 512 frames per sheet.');
  const frames = framesPerAsset(kind);
  if (frames > 1 && columns !== frames) throw new Error(`${kind === 'cast' ? 'Cast' : kind === 'door' ? 'Door' : 'Wall'} sheets need exactly ${frames} columns, with one asset per row.`);
  const crops: Crop[] = [];
  for (let row = 0; row < rows; row++) for (let col = 0; col < columns; col++) crops.push({ x: m + col * (cw + g), y: m + row * (ch + g), width: cw, height: ch });
  return { columns, rows, crops };
}

export function sheetManifest(settings: SheetSettings, frameCount: number): Pack {
  const s = settings;
  if (!/^[a-zA-Z0-9-]+$/.test(s.packId)) throw new Error('Pack ID may contain only letters, numbers, and hyphens.');
  if (!s.name.trim()) throw new Error('Give this library pack a name.');
  if (![s.footprintWidth, s.footprintHeight].every(n => Number.isInteger(n) && n >= 1 && n <= 8)) throw new Error('Footprints must be 1–8 cells on each side.');
  if (!Number.isInteger(s.pixelsPerWorldUnit) || s.pixelsPerWorldUnit < 1 || s.pixelsPerWorldUnit > 4096) throw new Error('Scale must be 1–4096 pixels per world unit.');
  if (![s.pivotX, s.pivotY].every(Number.isInteger) || s.pivotX < 0 || s.pivotY < 0 || s.pivotX >= s.cellWidth || s.pivotY >= s.cellHeight) throw new Error('The foot pivot must be inside the cell.');
  const group = framesPerAsset(s.kind);
  if (!Number.isInteger(frameCount) || frameCount < 1 || frameCount > 512 || frameCount % group !== 0) throw new Error('This sheet has incomplete asset frames.');
  const names = s.names.split('\n').map(name => name.trim()).filter(Boolean);
  const count = s.kind === 'ground' && s.variants ? 1 : frameCount / group;
  if (names.length && names.length !== count) throw new Error(`Enter exactly ${count} names, one per line, or leave names empty.`);
  const pack: Pack = { schemaVersion: 2, visualProfile: 'dimetric-30-v1', id: s.packId, name: s.name.trim(), pixelsPerCell: 128, filtering: 'linear', terrains: [], decals: [], walls: [], doors: [], props: [], themes: [], tiles: [], objects: [], cast: [], baseUrl: '' };
  const files = Array.from({ length: frameCount }, (_, i) => `frames/${i + 1}.png`);
  const frame = (i: number) => ({ file: files[i], sizePx: [s.cellWidth, s.cellHeight] as [number, number], pivotPx: [s.pivotX, s.pivotY] as [number, number] });
  for (let i = 0; i < count; i++) {
    const id = `asset-${i + 1}`;
    const name = names[i] || (count === 1 ? s.name.trim() : `${s.name.trim()} ${i + 1}`);
    if (s.kind === 'ground' || s.kind === 'overlay') {
      const tile: TileDef = { id, name, kind: s.kind, sourceSizePx: [s.cellWidth, s.cellHeight], coverageCells: [1, 1], files: s.kind === 'ground' && s.variants ? files : [files[i]], transitionPolicy: s.kind === 'ground' ? 'organic' : 'none' };
      pack.tiles!.push(tile);
    } else if (s.kind === 'cast') {
      const cast: CastDef = { id, name, pixelsPerWorldUnit: s.pixelsPerWorldUnit, suggestedFootprintCells: [s.footprintWidth, s.footprintHeight], views: { north: frame(i * 4), east: frame(i * 4 + 1), south: frame(i * 4 + 2), west: frame(i * 4 + 3) } };
      pack.cast!.push(cast);
    } else {
      const object: ObjectDef = { id, name, kind: s.kind === 'object' ? 'static' : s.kind, placement: s.kind === 'object' ? 'cell' : 'edge', footprintCells: [s.footprintWidth, s.footprintHeight], pixelsPerWorldUnit: s.pixelsPerWorldUnit, blocksMovement: s.blocksMovement, blocksVision: s.blocksVision, occludesVisually: true,
        renderLayers: Array.from({ length: group }, (_, j) => ({ kind: 'upright', ...frame(i * group + j), axis: s.kind === 'object' ? '' : j < (s.kind === 'door' ? 2 : 1) ? 'h' : 'v', state: s.kind === 'door' ? j % 2 === 0 ? 'closed' : 'open' : '' })) };
      pack.objects!.push(object);
    }
  }
  return pack;
}

/** Minimal standard ZIP writer: PNGs are already compressed. CRCs are checked by the server. */
export function sheetZip(entries: { name: string; bytes: Uint8Array }[]): Uint8Array<ArrayBuffer> {
  const encoder = new TextEncoder();
  const locals: Uint8Array[] = [], central: Uint8Array[] = [];
  let offset = 0, centralSize = 0;
  for (const entry of entries) {
    const name = encoder.encode(entry.name);
    const crc = crc32(entry.bytes);
    const local = new Uint8Array(30 + name.length + entry.bytes.length);
    const lv = new DataView(local.buffer);
    lv.setUint32(0, 0x04034b50, true); lv.setUint16(4, 20, true); lv.setUint16(6, 0x800, true);
    lv.setUint32(14, crc, true); lv.setUint32(18, entry.bytes.length, true); lv.setUint32(22, entry.bytes.length, true); lv.setUint16(26, name.length, true);
    local.set(name, 30); local.set(entry.bytes, 30 + name.length); locals.push(local);
    const cd = new Uint8Array(46 + name.length); const cv = new DataView(cd.buffer);
    cv.setUint32(0, 0x02014b50, true); cv.setUint16(4, 20, true); cv.setUint16(6, 20, true); cv.setUint16(8, 0x800, true);
    cv.setUint32(16, crc, true); cv.setUint32(20, entry.bytes.length, true); cv.setUint32(24, entry.bytes.length, true); cv.setUint16(28, name.length, true); cv.setUint32(42, offset, true); cd.set(name, 46);
    central.push(cd); centralSize += cd.length; offset += local.length;
  }
  if (offset + centralSize > 100 * 1024 * 1024) throw new Error('The cropped pack exceeds 100 MiB. Split the sheet into smaller packs.');
  const end = new Uint8Array(22); const ev = new DataView(end.buffer);
  ev.setUint32(0, 0x06054b50, true); ev.setUint16(8, entries.length, true); ev.setUint16(10, entries.length, true); ev.setUint32(12, centralSize, true); ev.setUint32(16, offset, true);
  const result = new Uint8Array(offset + centralSize + end.length); let pos = 0;
  for (const part of [...locals, ...central, end]) { result.set(part, pos); pos += part.length; }
  return result;
}
function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

export async function cropSheet(image: HTMLImageElement, settings: SheetSettings): Promise<File> {
  const grid = sheetGrid(image.naturalWidth, image.naturalHeight, settings);
  const manifest = sheetManifest(settings, grid.crops.length);
  const entries = [{ name: 'pack.json', bytes: new TextEncoder().encode(JSON.stringify(manifest)) as Uint8Array }];
  const canvas = document.createElement('canvas'); canvas.width = settings.cellWidth; canvas.height = settings.cellHeight;
  const ctx = canvas.getContext('2d'); if (!ctx) throw new Error('This browser cannot crop sheets.');
  for (const [index, crop] of grid.crops.entries()) {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(image, crop.x, crop.y, crop.width, crop.height, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise<Blob>((resolve, reject) => canvas.toBlob(blob => blob ? resolve(blob) : reject(new Error('Could not encode a sheet cell.')), 'image/png'));
    entries.push({ name: `frames/${index + 1}.png`, bytes: new Uint8Array(await blob.arrayBuffer()) });
  }
  return new File([sheetZip(entries)], `${settings.packId}.zip`, { type: 'application/zip' });
}
