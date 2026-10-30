// ------------------------------------------------------------
// Module: tileVariant
// Purpose: Pick a tile picture for one cell. The same seed, cell,
//          and asset always pick the same picture, and a neighbor
//          edit cannot change it. A stored byte can keep another
//          picture: 0 follows the seed, and 1 is the first file.
// ------------------------------------------------------------

export function tileVariant(seed: number, x: number, y: number, assetId: string, count: number): number {
  if (count <= 1) return 0;
  let hash = seed >>> 0;
  hash = Math.imul(hash ^ (x >>> 0), 0x9e3779b1);
  hash = Math.imul(hash ^ (y >>> 0), 0x85ebca6b);
  for (let index = 0; index < assetId.length; index += 1) {
    hash = Math.imul(hash ^ assetId.charCodeAt(index), 0xc2b2ae35);
  }
  hash = (hash ^ (hash >>> 16)) >>> 0;
  return hash % count;
}

/** The file a cell draws. A stored byte outside the pack falls back to the seed. */
export function pictureIndex(seed: number, x: number, y: number, assetId: string, count: number, stored: number): number {
  if (count <= 1) return 0;
  if (stored > 0 && stored <= count) return stored - 1;
  return tileVariant(seed, x, y, assetId, count);
}

/** The next stored byte. Landing on the seed's picture clears the pin. */
export function nextStoredVariant(seed: number, x: number, y: number, assetId: string, count: number, stored: number): number {
  if (count <= 1) return 0;
  const next = (pictureIndex(seed, x, y, assetId, count, stored) + 1) % count;
  return next === tileVariant(seed, x, y, assetId, count) ? 0 : next + 1;
}

/** Local picture position inside one cell. 0 is the west or north edge. */
export function tileUv(mapX: number, mapY: number): [number, number] {
  return [mapX - Math.floor(mapX), mapY - Math.floor(mapY)];
}
