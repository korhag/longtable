# How to draw assets for the table

The live scene uses one camera, `dimetric-30-v1`. It is orthographic, 30° above the ground, and looks from the south-west. Pan and zoom do not turn it. A new pack uses schema 2. The GM imports one zip. The same records paint the ground, place objects, and dress the cast. Do not copy a new picture into the old terrain or prop lists.

An example manifest is `docs/schemas/pack-v2.example.json`.

## What a pack is

```json
{
  "schemaVersion": 2,
  "id": "demo",
  "name": "Demo",
  "visualProfile": "dimetric-30-v1",
  "pixelsPerCell": 128,
  "filtering": "linear"
}
```

`id` is the pack name in every reference, as `demo/floor`. It cannot change after the first install. A second zip with the same id and the same bytes is ignored. Different bytes are refused, and nothing from that zip is installed.

`filtering` is `linear` or `nearest`. `pixelsPerCell` is 128 for painted ground. An upright picture uses its own `pixelsPerWorldUnit`.

Paths in the manifest are relative to the zip root. A path may not start with a slash, may not contain `..`, and may not be a link. The archive is at most 100 MiB compressed and 512 MiB expanded, with at most 2000 files. A picture is a PNG, at most 2048 pixels on a side.

## Ground

A ground tile is one cell.

| Field | Value |
| --- | --- |
| `kind` | `ground` |
| `sourceSizePx` | `[128, 128]` |
| `coverageCells` | `[1, 1]` |
| `files` | one or more PNGs |
| `transitionPolicy` | `none`, `rigid`, or `organic` |

The picture is drawn looking straight down. The camera shortens the north-south direction. Do not paint that shortening into the file. `rigid` keeps the picture aligned to the cell. `organic` only changes which neighbor the edge borrows. It does not warp the picture. Extra files are variants. The map seed picks one and keeps it.

A `side` tile is the same picture used for a future raised edge. The table does not raise cells yet.

An `overlay` tile is a flat picture on top of the ground, such as a rug or a puddle. It is not a wall.

## Objects

An object is one placed thing: a barrel, a tree, a wall, or a door.

| Field | Allowed |
| --- | --- |
| `kind` | `static`, `wall`, or `door` |
| `placement` | `cell` or `edge` |
| `footprintCells` | each side from 1 to 8 |
| `pixelsPerWorldUnit` | 1 to 4096 |

`kind: stairs` and `placement: connection` are refused. The message says raised terrain and stairs are not available yet.

Each picture is a `renderLayers` entry:

| Field | Meaning |
| --- | --- |
| `kind` | `upright` or `ground` |
| `file` | PNG path |
| `sizePx` | width and height of that file |
| `pivotPx` | the pixel that sits on the object's root |
| `rootOffsetWorld` | optional shift from the root, at most 64 cells on each axis |
| `axis` | `""`, `h`, or `v` for a wall or door |
| `state` | `""`, `open`, or `closed` for a door |

The pivot is a pixel index from the top-left of the file. `0` is the first pixel. The last valid index is one less than the size. A pivot equal to the size is refused. An omitted `originCells` puts the root at the center of the footprint. `[0, 0]` is the north-west corner.

An upright picture stands on the root and faces the camera. It is not a flat card turned with the camera quaternion. A ground layer lies on the cell. Draw the object on a transparent background. An opaque upright picture is accepted with a warning, because the outline and the depth cut need real transparency.

`blocksMovement` without `blocksVision` stops walking and does not stop sight. `blocksVision` becomes a wall of the vision polygon. On this flat table that wall also stops walking. A tree should use a small trunk polygon for both, and let the picture extend past it. Finite height, and a canopy that blocks sight without blocking feet, are not implemented.

`collisionPolygon` and each vision polygon are in world units around the root, with 3 to 32 points. An empty collision polygon uses the footprint rectangle.

A door or wall on an edge needs a layer for each axis it should show. `h` runs east-west. `v` runs north-south. `state` picks the open or closed picture. An empty axis or state matches both.

## Cast

A cast entry has four views: `north`, `east`, `south`, and `west`. Each view has a `file`, `sizePx`, and `pivotPx`, with the same pivot rule as an object.

North is toward the top of the map, which is negative Z. East is positive X. South is positive Z. West is negative X. When a figure moves, the larger step picks the facing. A tie keeps the previous facing.

The portrait is a separate file. It is the picture on the sheet and on a token that is not standing on a built map. It is not the body on the map. Assigning a cast picture does not replace the portrait.

On a built map the body comes from the token's cast picture when one is set, otherwise from the character's cast picture, otherwise from the token's own picture. A hidden figure is left out of a player's view entirely: no body, no outline, no name, and no click.

## What the table will refuse

- A profile other than `dimetric-30-v1`.
- A ground tile that is not 128 by 128, or that covers more than one cell.
- Stairs, a connection placement, or a height grid that is not all ground level.
- A missing picture, a pivot outside the picture, a zero footprint, or a scale outside 1 to 4096.
- A reference the catalog does not contain. The map will not skip that blocker.
- A zip that breaks the size limits, or a path that escapes the pack.

## What is still later work

Raised tiers, stairs, and sight that knows height are not part of this pack. A flat scene can be imported, painted, published, joined, saved, and reloaded. An elevated scene cannot. See the remaining goals in `docs/milestones/flat-gate.md`.
