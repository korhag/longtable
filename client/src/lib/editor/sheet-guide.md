# Longtable asset sheet guide

Use Map → Library → Import sheet. Download this guide there whenever you generate a new set.

## Art direction

Weathered dark fantasy: moss, damp earth, worn stone, bark, muted olive foliage, warm leather, restrained brass. Paint readable shapes with a coherent soft overhead light. Ground is top-down. Props and people are upright cutouts for a fixed orthographic camera 30° above the ground, looking from the south-west. Never add perspective to the ground tiles.

## Sheet contract

Deliver PNG, with a regular grid and identical cell dimensions. No titles, labels, guides, frames, borders, watermark, or baked checkerboard. Zero gutter and outer margin is easiest; otherwise enter their exact pixel widths in the importer. Keep the image at most 8192 pixels per side, 16 megapixels total, 32 MiB, and 512 frames. Each cropped cell must be at most 2048 pixels per side. The importer previews every crop; it does not guess where irregularly arranged art begins or remove backgrounds.

| Kind | Cell | Layout | Background |
| --- | --- | --- | --- |
| Ground | 128 × 128 px | Any regular grid, read left-to-right then top-to-bottom | Opaque seamless texture |
| Overlays | Usually 128 × 128 px | One overlay per cell | Transparent |
| Props | Usually 256 × 512 px | One whole object per cell | Transparent |
| Cast | 128 × 256 px | Exactly four columns: North, East, South, West; one character per row | Transparent |
| Walls | 128 × 256 px | Exactly two columns: horizontal edge, vertical edge; one wall style per row | Transparent |
| Doors | 128 × 256 px | Exactly four columns: H closed, H open, V closed, V open; one door style per row | Transparent |

## Natural ground

Draw straight down, without edge outlines or cast shadows. Repeat every tile seamlessly on all four edges. Keep surface detail distributed without a distinctive central motif. Make 4–8 restrained variants of each material: forest litter, packed soil, mossy cobbles. In the importer, enable “One terrain with natural variants” for one material: the brush automatically chooses stable variations. Disable it if each cell is a different material. Organic terrain transitions are assigned automatically. Keep neighboring materials similar in value and scale so their edges blend naturally. Smooth transitions cannot hide decorative tile borders baked into your artwork.

Example generation brief: “A 4 by 2 contact sheet of seamless 128 by 128 top-down dark fantasy forest floor texture variants. Moss, small roots, muted fallen leaves and damp soil, consistent diffuse light, subdued olive and umber palette, equal grid with zero gutter. Texture reaches every cell edge. No labels, no borders, no perspective.”

## Upright props

Show the entire tree, rock, or prop inside its transparent cell. Leave a little breathing room at the sides. Every picture in a sheet uses the same foot/root pivot; the default is horizontal center at the last pixel row. Align roots and feet to that position, or enter another pivot in Size, foot pivot & collision. Default scale is 128 pixels per world unit. A 256-pixel wide tree at that scale is two world units wide; its footprint can still be smaller than the canopy.

Footprints are the occupied ground cells, 1–8 per axis. Movement and sight blocking are separate options. The simple importer uses a rectangular footprint for collision. Trees with narrow trunks and broad canopies, separate canopy layers, low hedges with explicit vision height, and other custom volumes should use a schema-2 ZIP manifest; see docs/assets.md. Do not make all decorative foliage block sight.

## Cast

Four matching views per character: north/back, east/right, south/front, west/left. Keep clothing, equipment, proportions, scale and foot pivot identical across views. Each row is a different character. No names or direction letters within the images. A 4-column by 4-row sheet can hold Human, Elf, Dwarf and Orc. Enter those names on separate lines. Cast images do not overwrite existing character portraits.

For drawn animations, make one separate transparent atlas per direction. A recommended atlas is 1024 × 1536 pixels: four 256 × 256 frames per row, with rows idle, melee, ranged, move, hurt and spell in that order. Use zero gutter and margin, no labels, and the same frame-local foot pivot in every cell. Import these four atlases through **Import pack**, with the optional `animations` metadata described in docs/assets.md; the simple Cast sheet importer creates static directional views. The pack keeps its four static views for previews and includes frame dimensions, pivot, scale, row indices, frame counts and FPS for playback.

Animation directions use objects, for example `"east": {"file": "cast/elf/animation-east.png", "pixelsPerWorldUnit": 96.75, "pivotPx": [128,224]}`. Only `file` is required; omitted density and pivot inherit the shared animation defaults. Directional density accepts fractional finite values from 1–4096. If you shrink a direction's drawings to fit broad weapon poses inside its cells, reduce its density proportionally to preserve the character's world height. Keep every complete pose and effect within its own cell, with transparent padding; align all feet in that direction to the declared pivot.

## Walls and doors

Draw both fixed-camera axes. Wall segments represent one grid edge. Align ends so repeated pieces meet without gaps. Doors need four frames, including a visibly open leaf and clear doorway; all four share the same pivot and canvas size. Walls and closed doors normally block movement and sight; the game handles opening the door. Import roof art separately so it does not cover the room permanently.

## Import and revise

1. Choose the asset kind and PNG. Adjust dimensions, gutter and margin until the preview fits every frame exactly.
2. Enter a pack name and unique ID using letters, numbers and hyphens. Optionally enter one name per asset, one per line.
3. Confirm size, pivot and collision, then Add assets to library. The server validates the generated pack before installing anything.
4. Select a tile, prop, wall, door or cast entry in the library to use it.
5. For new art revisions use a new pack ID. An identical import is safe to repeat; different content under an existing ID is rejected to preserve scenes that already reference it.

Complex multi-layer assets, explicit collision/vision polygons and handcrafted transition sets remain available through Import pack with the schema-2 format described in docs/assets.md.
