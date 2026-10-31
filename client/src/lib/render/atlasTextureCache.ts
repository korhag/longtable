import * as THREE from 'three';

export interface AtlasLease {
  ready: Promise<THREE.Texture | null>;
  release: () => void;
}
type Entry = { ready: Promise<THREE.Texture | null>; texture: THREE.Texture | null; references: number; used: number; retired: boolean; settled: boolean };
type Loader = (url: string, filtering: 'linear' | 'nearest') => Promise<THREE.Texture | null>;

function loadAtlas(url: string, filtering: 'linear' | 'nearest'): Promise<THREE.Texture | null> {
  return new Promise(resolve => {
    new THREE.TextureLoader().load(url, texture => {
      texture.colorSpace = THREE.SRGBColorSpace;
      texture.magFilter = texture.minFilter = filtering === 'nearest' ? THREE.NearestFilter : THREE.LinearFilter;
      texture.wrapS = texture.wrapT = THREE.ClampToEdgeWrapping;
      texture.generateMipmaps = false;
      resolve(texture);
    }, undefined, () => resolve(null));
  });
}

/** A hard atlas limit, including in-flight decodes. Visible figures share leases.
 * Unused least-recently-used sheets are disposed before a replacement is loaded.
 * When every slot is in use the caller keeps its existing still picture.
 */
export class AtlasTextureCache {
  private entries = new Map<string, Entry>();
  private clock = 0;
  private disposed = false;
  constructor(readonly capacity = 12, private loader: Loader = loadAtlas) {}

  acquire(url: string, filtering: 'linear' | 'nearest'): AtlasLease | null {
    if (this.disposed) return null;
    const key = `${filtering}:${url}`;
    let entry = this.entries.get(key);
    if (!entry) {
      if (this.entries.size >= this.capacity) {
        let oldest: [string, Entry] | null = null;
        for (const candidate of this.entries) {
          // TextureLoader requests cannot be aborted. Keep pending slots even
          // after their last actor turns away, until their decode has settled.
          if (candidate[1].settled && !candidate[1].references && (!oldest || candidate[1].used < oldest[1].used)) oldest = candidate;
        }
        if (!oldest) return null;
        this.retire(oldest[0], oldest[1]);
      }
      entry = { ready: Promise.resolve(null), texture: null, references: 0, used: ++this.clock, retired: false, settled: false };
      const created = entry;
      entry.ready = this.loader(url, filtering).then(texture => {
        created.settled = true;
        if (created.retired) { texture?.dispose(); return null; }
        created.texture = texture;
        return texture;
      }).catch(() => { created.settled = true; return null; });
      this.entries.set(key, entry);
    }
    entry.references++;
    entry.used = ++this.clock;
    const acquired = entry;
    let released = false;
    return { ready: entry.ready, release: () => {
      if (released) return;
      released = true;
      acquired.references--;
      acquired.used = ++this.clock;
      if (!acquired.references && acquired.settled && !acquired.texture && !acquired.retired) this.retire(key, acquired);
    } };
  }

  private retire(key: string, entry: Entry) {
    entry.retired = true;
    entry.texture?.dispose();
    entry.texture = null;
    this.entries.delete(key);
  }

  get size() { return this.entries.size; }
  get inUse() { return [...this.entries.values()].filter(entry => entry.references > 0).length; }

  /** Called by the owning renderer on disposal; pending loads dispose on arrival. */
  dispose() { this.disposed = true; for (const [key, entry] of this.entries) this.retire(key, entry); }
}
