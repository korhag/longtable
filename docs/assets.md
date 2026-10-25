# How to draw assets for the table

This is the contract for every picture the map uses. A person or an image model can follow it. The editor does not care whether a picture is painted or pixel art. The pack says which, and the engine follows the pack.

Nothing here has to be pretty yet. A pack with only colors still builds a map. Pictures replace those colors when the files appear.

## The view

The camera looks **north**, down at **53.13 degrees** above the ground. It never turns and it never changes that angle. It can pan and zoom.

That angle is chosen so the numbers stay simple:

| Direction on the ground or in the air | How long it looks on screen |
| --- | --- |
| 1 cell east to west | P pixels |
| 1 cell north to south | 0.8 P pixels |
| 1 cell of height | 0.6 P pixels |

P is the pack's `pixelsPerCell`. Painted packs use **128**. Pixel-art packs can use a smaller P, with `filtering` set to `nearest`.

Use a P that is a multiple of 5 (40, 80, 160) when you want every upright edge on a whole pixel. With P = 128 the engine rounds to the nearest pixel, which is at most half a pixel off. The formulas below are the source of truth. `round` means round to the nearest integer, halves away from zero.

```
upright width  = round(footprintWidth * P)
upright height = round(footprintDepth * 0.8 * P + height * 0.6 * P)
```

```
        camera
          \
           \  53.13°
            \
             *---------- north, top of the map
            ground
```

Screen right is east. Screen up is mostly north, with a little world-up in it. The engine's screen-up direction is `(0, 0.6, -0.8)` and the camera sits `0.8` up and `0.6` south of the point it looks at, times the zoom distance.

Two kinds of picture:

- **Ground art** (terrain and decals) is drawn looking straight down. The engine lays it flat. The tilt shortens the north-south direction by itself. Do not draw that shortening into the picture.
- **Upright art** (props, walls, doors, and later characters) is drawn from the camera's angle, on a transparent background. The engine puts it on a flat card that faces the camera. One pixel in the file is one pixel on screen at the pack's native zoom, so the card is `imageWidth / P` world units wide and `imageHeight / P` tall.

The **anchor** is the pixel that sits on the front-left corner of the object's footprint. Front means the south edge, the edge nearest the camera. Left means west. Anchor coordinates are pixels from the top-left of the file, x to the right, y down. The bottom edge of the file is `y = imageHeight`.

Pixels left of the anchor hang to the west of the footprint. Pixels above it are the object and its north side. Do not put opaque pixels below the anchor. That area would float in front of the footprint, toward the camera.

## Global rules

- PNG with a real alpha channel. WebP is also accepted. Color is sRGB.
- Light comes from the **northwest**, high up. Shadows fall toward the **southeast** and are painted into the picture. The engine does not add a shadow.
- No text, no frame, no drop shadow outside the object, no ground plane behind it.
- Upright pictures are mostly transparent. The object is the only opaque part.
- Do not draw a perspective other than the one stated for that category.
- File names are lowercase, with hyphens. Paths below are relative to the pack folder.
- A missing file is not an error. Terrain falls back to its `swatch` color. An upright piece falls back to a box the size of its footprint, with its name.

## Folder

```
assets/packs/<pack>/pack.json
assets/packs/<pack>/terrain/<id>.png
assets/packs/<pack>/terrain/<id>-2.png
assets/packs/<pack>/decals/<id>.png
assets/packs/<pack>/walls/<id>-front.png
assets/packs/<pack>/walls/<id>-side.png
assets/packs/<pack>/walls/<id>-post.png
assets/packs/<pack>/doors/<id>-front-closed.png
assets/packs/<pack>/doors/<id>-front-open.png
assets/packs/<pack>/doors/<id>-side-closed.png
assets/packs/<pack>/doors/<id>-side-open.png
assets/packs/<pack>/props/<id>.png
assets/packs/<pack>/props/<id>-2.png
assets/packs/<pack>/tray/felt.png
assets/packs/<pack>/tray/wood.png
```

An id is unique inside its category. On a map, and in a theme's scatter list, an asset is written `pack/id`, for example `core/oak`.

## Terrain

Seamless, straight down, covering **4 by 4 cells**.

| P | File size |
| --- | --- |
| 128 | 512 × 512 |
| 32 | 128 × 128 |
| 160 | 640 × 640 |

The engine repeats the picture every four cells and blends one terrain into the next with noise. **Do not paint the border between grass and dirt.** A `layer` number says which one creeps over the other. A higher layer spreads over a lower one, the way grass grows over dirt. Water stays low.

Each terrain has a base file and up to three variants (`<id>.png`, `<id>-2.png`, `<id>-3.png`, `<id>-4.png`). Variants are optional. `swatch` is the flat color used until a file exists, written `#rrggbb`.

Prompt:

```
Seamless top-down texture of <terrain>, <size> pixels square, tileable on all
four edges, no perspective, no horizon, no text, even daylight, sRGB PNG.
```

## Decals

Small ground details, also drawn straight down: a tuft, a crack, a puddle, leaves. They are not seamless. `size` is the footprint in cells, `[width, depth]`.

```
file width  = round(size[0] * P)
file height = round(size[1] * P)
```

`rotate` means the editor may turn the decal when it scatters it. Only set it when the picture has no obvious north.

Prompt:

```
A single <thing> seen straight from above, <width> by <height> pixels,
transparent background, no shadow outside the shape, sRGB PNG with alpha.
```

## Walls

Walls are **1 cell high** and **0.25 cell thick**. A theme's kit has three pictures so a run of wall connects around corners. The engine places `front` on an east-west edge, `side` on a north-south edge, and `post` on a corner where those two meet. All three share the same height and the same thickness, which is what makes them meet.

| Piece | Footprint (width × depth) | P = 128 | P = 160 (exact) | Anchor |
| --- | --- | --- | --- | --- |
| `front`, east to west, 1 cell long | 1 × 0.25 | 128 × 102 | 160 × 128 | bottom-left |
| `side`, north to south, shows the top and the south end | 0.25 × 1 | 32 × 179 | 40 × 224 | bottom-left |
| `post`, the corner | 0.25 × 0.25 | 32 × 102 | 40 × 128 | bottom-left |

Bottom-left means the anchor is `[0, imageHeight]`.

Draw `side` as the south end of the wall plus the top receding north. Do not draw the long east or west face. The camera almost cannot see it.

Prompt:

```
A 1-cell stone wall section seen from the south, camera 53 degrees above the
ground looking north. Canvas <width> by <height> pixels, transparent
background. The base of the wall sits on the bottom edge. Light from the
northwest. No text. sRGB PNG with alpha.
```

## Doors

A door has four pictures: `front` and `side`, each **closed** and **open**. Each one is the same pixel size as the wall piece it replaces, so a door sits in a wall without a gap. The open picture shows the doorway you can see through, with the door leaf swung aside. The closed picture shows the shut door in the same frame.

Name the wall kit the door belongs to with `wall`, so the editor offers that door next to that wall.

## Props

A prop is anything that stands on the ground: a table, a barrel, a tree, a chest.

| Field | Meaning |
| --- | --- |
| `footprint` | `[width, depth]` in cells. The rectangle the prop occupies. |
| `blocks` | `none`, `move`, or `sight`. `sight` also blocks walking. `move` blocks walking but not vision, which is what a table does. |
| `height` | How tall it is, in cells. Used for the canvas and for the fallback box. |
| `anchor` | The pixel of the footprint's front-left ground corner. |
| `mirror` | True when flipping the picture left to right still looks right. |
| `file` and `variants` | `<id>.png`, then optional `<id>-2.png` and so on. |

The canvas is at least the upright size. It may be wider, for a canopy or a shadow that should stay inside the file. Keep the anchor on the front-left corner even when the canvas is larger than the footprint.

### A table, 2 by 1, 0.75 cell tall, P = 128

```
width  = round(2 * 128) = 256
height = round(1 * 0.8 * 128 + 0.75 * 0.6 * 128) = round(160) = 160
anchor = [0, 160]
blocks = move
```

The tabletop's near edge lies on the bottom of the file. The far edge is `0.8 * 128 = 102` pixels above the bottom. The top of the tabletop is another `0.45 * 128 = 58` pixels up.

### A tree, 2 by 2, 3 cells tall, P = 128

```
width  = 256
height = round(2 * 0.8 * 128 + 3 * 0.6 * 128) = round(435.2) = 435
anchor = [0, 435]
blocks = sight
```

The trunk stands inside the 2 by 2 square. The canopy may spread wider than the footprint. If it does, grow the canvas to the sides and move the anchor's x so the trunk's front-left ground point is still the anchor.

Prompt:

```
A <object>, seen from the south with the camera 53 degrees above the ground
looking north. Canvas <width> by <height> pixels, transparent background.
The front-left ground corner of its <w> by <d> cell footprint is the pixel
at <anchor>. Light from the northwest, shadow toward the southeast, painted
into the image. No text, no ground plane. sRGB PNG with alpha.
```

## Themes

A theme is what a brush stroke means. It names one terrain and a scatter list. Each scatter entry is an asset id (`core/oak`), a `density` per 10 painted cells, and a `spacing` in cells. The editor paints the terrain, then places about `density` of that asset for every 10 cells, never closer than `spacing`, never on top of another footprint, and never across a door.

`wall` and `door` name the kit the wall tool uses while that theme is selected.

## Dice tray

Two seamless pictures, straight down, any square size that is a multiple of 64. `tray/felt.png` fills the tray floor. `tray/wood.png` fills the wooden ring. Until the files exist, the tray keeps its flat felt and wood colors.

Prompt:

```
Seamless <felt or wood grain> texture, 512 pixels square, tileable, straight
top-down, no objects, sRGB PNG.
```

## Characters, later

Tokens are discs for now. When a character becomes a picture, draw it as upright art, one cell wide and deep, about 1.5 cells tall:

```
width  = P
height = round(0.8 P + 1.5 * 0.6 P) = round(1.7 P)
```

At P = 128 that is 128 × 218, anchor `[64, 218]` so the figure stands in the middle of the cell. The same light and the same camera as a prop.

## pack.json

```json
{
  "id": "core",
  "name": "Core",
  "pixelsPerCell": 128,
  "filtering": "linear",
  "terrains": [
    { "id": "grass", "name": "Grass", "swatch": "#5d8a4a", "layer": 2, "files": ["terrain/grass.png"] }
  ],
  "decals": [
    { "id": "tuft", "name": "Tuft", "file": "decals/tuft.png", "size": [1, 1], "rotate": true }
  ],
  "walls": [
    {
      "id": "hedge",
      "name": "Hedge",
      "front": "walls/hedge-front.png",
      "side": "walls/hedge-side.png",
      "post": "walls/hedge-post.png"
    }
  ],
  "doors": [
    {
      "id": "hedge-gate",
      "name": "Hedge gate",
      "wall": "hedge",
      "frontClosed": "doors/hedge-gate-front-closed.png",
      "frontOpen": "doors/hedge-gate-front-open.png",
      "sideClosed": "doors/hedge-gate-side-closed.png",
      "sideOpen": "doors/hedge-gate-side-open.png"
    }
  ],
  "props": [
    {
      "id": "table",
      "name": "Table",
      "file": "props/table.png",
      "variants": [],
      "footprint": [2, 1],
      "blocks": "move",
      "anchor": [0, 160],
      "mirror": true,
      "height": 0.75
    }
  ],
  "themes": [
    {
      "id": "forest",
      "name": "Forest",
      "terrain": "grass",
      "wall": "hedge",
      "door": "hedge-gate",
      "scatter": [
        { "asset": "core/oak", "density": 1.6, "spacing": 3 },
        { "asset": "core/tuft", "density": 3, "spacing": 1 }
      ]
    }
  ],
  "tray": { "felt": "tray/felt.png", "wood": "tray/wood.png" }
}
```

`filtering` is `linear` or `nearest`. Pixel art uses `nearest`, and the camera then keeps each cell on a whole number of screen pixels.

The server rejects a pack that does not parse, that repeats an id, that names a terrain or scatter asset it does not contain, or whose `pixelsPerCell` is outside 8 to 512. The message says what is wrong.

## The core pack still needs pictures

`assets/packs/core/pack.json` lists these. None of the files are drawn yet. The swatches and the footprint boxes are enough to build a map, and this list is the drawing queue.

Terrains: `grass`, `dirt`, `sand`, `snow`, `ice`, `swamp`, `water`, `stone`.

Decals: `tuft`, `crack`, `pebble`, `snow-drift`, `puddle`, `moss`.

Walls: `hedge`, `palisade`, `sandstone`, `ice-wall`, `bog-fence`, `masonry`.

Doors: `hedge-gate`, `palisade-gate`, `sand-arch`, `ice-gate`, `bog-gate`, `dungeon-door`.

Props: `oak`, `pine`, `bush`, `rock`, `cactus`, `dead-tree`, `reed`, `ice-spike`, `crate`, `barrel`, `table`, `pillar`.

Themes, and what they scatter:

| Theme | Terrain | Places |
| --- | --- | --- |
| Grassland | grass | bush, rock, tuft |
| Forest | grass | oak, pine, bush, tuft |
| Desert | sand | cactus, rock, crack |
| Glacier | snow | ice-spike, rock, snow-drift |
| Swamp | swamp | dead-tree, reed, puddle |
| Dungeon | stone | crate, barrel, table, pillar, moss |
