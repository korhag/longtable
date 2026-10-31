# The Old Forest Road

Bundled woodland starter pack for Longtable's fixed dimetric camera. Original generated artwork follows the supplied dark woodland moodboard: moss, weathered timber, muted gold and stone.

Includes four ground materials (forest and road each have four seeded variants), four tree species, a boulder, timber walls, open/closed doors, steps, and eight characters with 32 distinct directional illustrations. Every character has north/back, east/right, south/front and west/left artwork, aligned to a consistent foot pivot.

The characters also include 32 newly drawn animation sheets: one per character and direction, each containing 24 frames in a 4-column, 6-row grid. Rows are idle, melee, ranged, move, hurt and spell. That is 768 animation frames. Generated poses are extracted, uniformly scaled and padded to 256×256 cells with a consistent foot anchor; the artwork is not created by deforming a still picture. The manifest supplies timing, scale and sheet paths. Runtime loading uses a bounded cache.

Load the playable encounter with **Edit → Map → Load forest encounter**. The previous scene stays in the campaign. The sample contains tiers 0, 1 and 2 linked by stairs, an enclosed cabin, four adventurers and four bandits.

Select the **Ancient woodland** biome to brush or fill the ground with deterministic variants and spaced trees. Ground-only painting is useful for roads. The room tool draws a perimeter; place a door on an edge to replace that wall segment. During play, visible figures inside a room lower its foreground walls. Fog still controls which figures are available to render.

For new sheets, use **Map → Import sheet** and download the guide from that dialog. PNG sheet cells must have consistent dimensions; upright art needs transparent backgrounds. See `docs/assets.md` for the schema contract.
