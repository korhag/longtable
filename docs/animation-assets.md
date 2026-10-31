# Starter character animations and asset size

The forest showroom has **8 characters × 4 directions × 6 actions × 4 drawn frames = 768 animation frames** across **32 PNG sheets**. It retains the original 32 directional still images for loading and fallback, making 64 character image files total.

Each sheet contains drawn idle, melee, ranged, move, hurt and spell poses. Sheet extraction aligns the feet and uniformly scales/pads the artwork into regular cells. Per-direction pixel density preserves the character's world size when wide weapon poses need more room. The engine advances frame UVs on a simple two-triangle card. Packs without animation sheets can still use the compact procedural fallback.

## Before and after

Measurements use decimal MB (1 MB = 1,000,000 bytes), on-disk sizes before and after this animation update. They measure bundled asset files, not JavaScript, fonts, saved campaigns or GPU memory.

| Bundled assets | Files before → after | Before | After | Added |
| --- | ---: | ---: | ---: | ---: |
| Forest character artwork | 32 → 64 PNG | 1.276 MB | 33.793 MB | 32.517 MB |
| Complete forest pack, including characters | 61 → 93 | 3.736 MB | 36.268 MB | 32.532 MB |
| All installed packs | 88 → 120 | 3.752 MB | 36.284 MB | 32.532 MB |
| Combat audio and credits | 11 → 11 (8 Ogg) | 0.114 MB | 0.114 MB | 0 MB |
| All packs + combat audio | 99 → 131 | 3.866 MB | 36.398 MB | 32.532 MB |

The rows overlap: character artwork is included in the forest pack, which is included in all packs. The last row is the combined total, not the sum of every row. The new 32 animation sheets are exactly **32,517,227 bytes**. The character artwork including stills is **33,792,853 bytes**; the complete forest pack is **36,268,088 bytes**. The raw generated sheets total 73,292,864 bytes and are not required or included in the game pack; the aligned runtime sheets are committed to Git.

On-disk PNG size is different from texture memory. Each 1024×1536 RGBA atlas occupies 6 MiB decoded. Loading all 32 at once would require 192 MiB for their pixels alone. The renderer instead shares sheets and keeps at most 12 active or pending atlases: up to 72 MiB of atlas GPU pixels, plus decoded image and renderer overhead. Eight distinct visible starter characters normally use eight sheets (48 MiB). Mipmaps are disabled. Unused sheets are evicted; unavailable sheets use the existing still until a cache slot opens. Hidden tabs stop drawing, and stationary poses do not run a continuous animation frame loop.

To reproduce the asset totals from the repository root:

```python
from pathlib import Path

for label, folder in [
    ('forest pack', 'assets/packs/forest'),
    ('all packs', 'assets/packs'),
    ('combat audio', 'client/public/audio/combat'),
]:
    files = [p for p in Path(folder).rglob('*') if p.is_file()]
    size = sum(p.stat().st_size for p in files)
    print(label, len(files), size, f'{size / 1_000_000:.6f} MB')
```

Right-click a controllable character and choose **Animations** for local previews. Movement and attack/damage results trigger poses during play. Spellcasting is available as a preview; a rules-specific spell action is needed to automate a particular spell's mechanics.
