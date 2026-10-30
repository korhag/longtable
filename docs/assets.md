# How to draw assets for the table

The live scene uses one camera, `dimetric-30-v1`. It is orthographic, 30° above the ground, and looks from the south-west. Pan and zoom do not turn it. A new pack uses schema 2. The GM imports one zip. The same records paint the ground, place objects, and dress the cast. Do not copy a new picture into the old terrain or prop lists.

An example manifest is `docs/schemas/pack-v2.example.json`. The same file is the template. `docs/schemas/pack-v2.schema.json` lists the fields. Map → Check pack runs the table's rules on a zip and installs nothing. Import pack is the step that adds it. A playable diagnostic pack is `assets/packs/ruin`. Its pictures are colored blocks, not finished art. The 12 by 12 scene in the server test uses that pack: four grounds, a transparent overlay, an oak, a barrel, a table, walls on both axes, a door, tiers 0, 1, and 2, and two stairs.

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

The picture is drawn looking straight down. The camera shortens the north-south direction. Do not paint that shortening into the file. `rigid` keeps the picture aligned to the cell. `organic` only changes which neighbor the edge borrows. It does not warp the picture. Extra files are variants. The map seed picks one and keeps it. Right-click a square while the ground brush is selected to keep a different picture. Painting that square again lets the seed choose.

A `side` tile is the picture on a raised edge. A cell is tier 0, 1, or 2. One tier is one world unit. A stair is the only way from one tier to the next.

An `overlay` tile is a flat picture on top of the ground, such as a rug or a puddle. It is not a wall.

## Objects

An object is one placed thing: a barrel, a tree, a wall, or a door.

| Field | Allowed |
| --- | --- |
| `kind` | `static`, `wall`, `door`, or `stairs` |
| `placement` | `cell`, `edge`, or `connection` |
| `footprintCells` | each side from 1 to 8 |
| `pixelsPerWorldUnit` | 1 to 4096 |

A stair uses `kind: stairs` and `placement: connection`. It joins two neighboring squares one tier apart. A jump from tier 0 to tier 2 needs two stairs.

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

`blocksMovement` stops walking. `blocksVision` stops sight and does not stop walking. A `visionVolume` gives that sight span a base and a height. A ray sees over it only when the line clears the top. With no volume, the span is 8 units tall. An eye is one step above its square, and a figure is sampled slightly lower. Terrain is solid below its surface. A tree should use a small trunk polygon, and let the picture extend past it.

`collisionPolygon` and each vision polygon are in world units around the root, with 3 to 32 points. An empty collision polygon uses the footprint rectangle.

A door or wall on an edge needs a layer for each axis it should show. `h` runs east-west. `v` runs north-south. `state` picks the open or closed picture. An empty axis or state matches both.

## Cast

A cast entry has four views: `north`, `east`, `south`, and `west`. Each view has a `file`, `sizePx`, and `pivotPx`, with the same pivot rule as an object.

North is toward the top of the map, which is negative Z. East is positive X. South is positive Z. West is negative X. When a figure moves, the larger step picks the facing. A tie keeps the previous facing.

`suggestedFootprintCells` is the body on the ground. A one-cell figure stands on a square center. An even body, such as the ruin Brute at 2 by 2, stands on the corner between squares. The picture does not change that size. The whole body has to stand on one height. A move or a map update that would split it across two levels is refused, and the figure stays put.

The portrait is a separate file. It is the picture on the sheet and on a token that is not standing on a built map. It is not the body on the map. Assigning a cast picture does not replace the portrait.

On a built map the body comes from the token's cast picture when one is set, otherwise from the character's cast picture, otherwise from the token's own picture. A hidden figure is left out of a player's view entirely: no body, no outline, no name, and no click.

## What the table will refuse

- A profile other than `dimetric-30-v1`.
- A ground tile that is not 128 by 128, or that covers more than one cell.
- A height other than 0, 1, or 2, or a stair that does not join neighboring squares one step apart.
- A missing picture, a pivot outside the picture, a zero footprint, or a scale outside 1 to 4096.
- A reference the catalog does not contain. The map will not skip that blocker.
- A zip that breaks the size limits, or a path that escapes the pack.

## Build a raised scene

The ruin pack in `assets/packs/ruin` is already on a table that loads this checkout. Its pictures are colored blocks, not finished art. A zip of that folder can also be checked and imported. The buttons below are the ones on the table.

1. Open Map. Check pack reads the zip and installs nothing. Import pack adds it. The library lists Tiles, Objects, and Cast. Select a ground tile to see it repeated, and joined with another ground. Select an object or a figure to see a gold mark on the pivot. The checker behind a picture shows where it is transparent.
2. New makes a draft. Click a ground tile, then paint. Leaves is an overlay, so it selects Decals. Click the map to place it.
3. Click an object. An oak, a barrel, or a table selects Props. A wall selects Walls. A door selects Doors. Moving over the map shows the squares that object will cover. Click a square for a prop, or an edge for a wall or a door.
4. Raise, then Step or Rise, paints a square to tier 1 or 2. A climb of two tiers needs two edges. Stairs, then the stair picture, then click the edge between the two heights. A flat edge names the two squares and says to raise one side by one step first. If a later height change leaves a stair between squares that are no longer one step apart, that stair is named in the editor, and the map stays off the table.
5. On the character, click Picture. Then open Map, Cast, and the picture. That sets the body. A later picture chosen for a new figure does not change it. The portrait stays on the sheet.
6. Put on the table. If a figure would be left outside the map, standing in an object, or standing on two heights, their name is shown first and they stay where they are. An object that covers two heights, or hangs off the map, is named the same way and stays in the draft. The button waits until that list is clear. Switch to Play. Place, or Add, waits for a click on the map. The marker shows the squares, and the label names the height. Dragging a figure onto a stair lands on the nearer end. A drag that stops in the middle lands on the higher square. Select a figure and choose North, East, South, or West to turn them without moving. An adversary starts hidden. Select the one players should see and press Show. Add uses the cast picture last clicked in the library.
7. See as that player. A figure on the high ground can see over a low hedge. A wall still hides the far side. A hidden figure is absent: no body, no outline, no name, and no click. Your eyes returns to the GM view.
8. Each change asks the table to save. After that save, a restart keeps the map, the cast pictures, and the hidden figure.

Middle-drag pans. The wheel zooms. The view does not turn.
