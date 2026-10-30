# Longtable — fixed 30° dimetric asset and scene implementation plan

**Prepared:** 29 October 2026  
**Repository:** `korhag/longtable`  
**Inspected branch:** `main`  
**Inspected commit:** `59b07fb6d3af939c48362e2edeeda0b3543c053f` — `Release v0.12.2`  
**Review mode:** read-only source inspection. No repository files were changed. No application, browser rendering, or test suite was executed during this review.  
**Deliverable:** implementation instructions for a coding agent, not a claim that the proposed features already exist.

---

## 0. The goal to implement

Make Longtable a usable, asset-driven, fixed-camera 2.5D tabletop. Every playable scene must use the **same orthographic projection: elevation 30° above the horizontal ground, azimuth 45° using the existing `basis()` convention**. Allow pan, zoom, fit, and centering; never expose scene-camera rotation, tilt, perspective, or alternate projection modes.

The GM must be able to import a validated asset pack, create a scene, paint **Tiles**, place **Objects**, assign **Cast** artwork to PCs/NPCs/adversaries, and play that scene with correct placement, movement, fog, object occlusion, and outlines of camera-obscured permitted cast. Support **three terrain tiers total: 0, 1, and 2**, with stairs between them, without building a general stacked-storey or full-3D game engine.

Deliver working vertical slices, not just types, an importer without a UI, a camera demo, or a large refactor. A diagnostic asset pack and playable test scene are required so this work can be accepted before production artwork arrives.

### Execution instructions for the coding agent

1. Read this plan and the repository's current instructions. Compare the checkout to the inspected commit; re-check changed code rather than blindly applying stale assumptions.
2. Make a working branch. Implement goals G0–G10 in order, keeping the project buildable and preserving unrelated functionality. Record evidence after each goal.
3. Use existing Rust/Svelte/Three.js architecture. Reuse the generalized projection mathematics; do not replace the engine or add a second scene renderer.
4. Rust remains authoritative for state, permissions, asset validity, movement, and visibility. Regenerate the TypeScript bindings from Rust; do not manually edit generated bindings.
5. Decisions explicitly marked **required** are acceptance criteria. Names of new files and fields are proposed implementation names; adjust them coherently where existing architecture calls for it.
6. Do not stop at “the camera works.” The endpoint is the complete import → build → assign cast → publish → play → save → reload workflow.
7. If a goal cannot yet pass its acceptance tests, report the specific failing behavior and keep it open. Do not silently substitute placeholder behavior for a required feature.

---

## 1. Locked product decisions

| Topic | Required decision |
|---|---|
| Scene presentation | One fixed 30°/45° orthographic dimetric profile, named `dimetric-30-v1` |
| Camera controls | Pan, zoom, fit, and center only; neither rotation nor tilt |
| World grid | Keep a square logical grid. A diamond is its screen projection, not a new coordinate system |
| Art direction | Painterly dark fantasy, compatible with the approved concept direction; coherent lighting and camera-matched silhouettes |
| Tiles | Top-down source images; default ground tile is **128 × 128 actual pixels covering one logical cell** |
| Objects | Static scenery with separate visual, movement, and sight properties; walls, doors, trees, barrels, tables, etc. |
| Cast | Movable scene entities linked to optional character records; sprite artwork is separate from portraits and game statistics |
| Cast directions | Four world-facing views for the standard production contract. Static idle poses are enough; animation is not a prerequisite |
| Occlusion | Scenery can obscure cast. Outline the obscured part of the cast's real alpha silhouette, not its PNG rectangle |
| Information security | Outlines never reveal a token the server has withheld or explicitly hidden |
| Elevation | One walkable terrain surface per cell, at level 0, 1, or 2; one level equals one world unit |
| Connections | Explicit stairs between adjacent tiers; no automatic walking up cliffs |
| Rendering architecture | Lightweight 3D terrain/support geometry plus camera-matched 2D cutouts, in one Three.js scene pipeline |
| Dungeon Alchemist | Not part of the new authoring or runtime workflow |
| Existing data | Back up and migrate safely; preserve unsupported original assets/data and report conversion requirements |

**“One mode” means one scene presentation and data pipeline.** Keep the app's useful Edit, Play, Campaign, and Rules/Design workflows. This requirement does not remove those workflows or change the separate dice-tray rendering.

### Explicit non-goals for this implementation

No camera orbit, alternate viewpoints, runtime perspective, arbitrary flying, stacked interiors, bridges with playable space underneath, destructible buildings, full skeletal character animation, or a general physics engine. No automatic combat-rule bonuses just because a combatant stands higher. No requirement for users to build 3D models.

Equipment attachment points may be represented in the asset contract, but interchangeable weapons/armor and animation are a subsequent feature. They must not delay loading a four-view cast asset and using it on the table.

---

## 2. What the source inspection actually found

These findings are tied to the inspected commit. File names below are existing files unless identified as proposed.

| Area | Observed implementation | Consequence for this plan |
|---|---|---|
| Projection | `client/src/lib/render/projection.ts` has generalized `basis(azimuthDeg, elevationDeg)` and `panGround()`. `view.ts` defines the old 0.8/0.6 projection | Reuse the generalized math; eliminate old hard-coded assumptions from the scene renderer |
| Camera | `render/engine.ts::placeCamera()` branches between imported presentation cameras and a fixed 53.13° north-facing built-map view | Collapse to one fixed profile rather than adding another camera branch |
| Framing and picking | `fitMap()` assumes old projected dimensions; `hitMap()` intersects a single ground plane; drag placement uses ground height zero | Update framing to projected 3D bounds and picking to elevated walkable surfaces |
| Terrain | `render/terrain.ts` uses a 128px internal texture layer, loads `files[0]`, and uses `fract(warped / 4.0)` | The current 512px/4-cell documentation is not the new contract. Implement real one-cell coverage, deterministic variants, and accurate material transitions |
| Scenery | `render/standees.ts` fixes RIGHT/UP/TOWARD to the old view, places props relative to a front-left footprint point, and uses one image per prop | The old orientation and anchors cannot simply be retained under a diagonal camera |
| Object metadata | `server/src/assets.rs` / `client/src/lib/assets/catalog.ts` contain terrains, decals, walls, doors, props, themes, and tray fields; no cast definition | Add a versioned asset contract and expose the three user-facing categories |
| Pack loading | The server loads packs from disk. The client catalog is cached. Current manifest validation does not validate all image files or the full proposed contract | Add a real import/validation/refresh workflow with actionable errors |
| Collision | `server/src/map/compile.rs` turns a prop's entire rectangular footprint into 2D segments. `sight` also means movement blocking | Separate art bounds, physical footprint, movement blocking, and finite-height sight blocking |
| Tokens | `TokenView` carries a portrait URL and character link, but no facing or cast asset reference. The renderer shows discs on ordinary maps and a fixed-size portrait card on imported presentations | Implement a dedicated cast layer and do not stretch character portraits into body sprites |
| Terrain levels | `server/src/map/layout.rs` has terrain cells, props, pieces, decals, lights, but no surface-level data | Elevation needs stored scene data and authoritative rules, not only a renderer offset |
| Visibility | `state.rs::vision_polygons()` and `reveal_from_tokens()` use 2D visibility polygons. `permissions.rs` filters tokens server-side | Keep permission filtering; extend visibility for finite-height obstacles and tiered terrain |
| Editor | `editor/draft.svelte.ts` already has painting, undo/redo, draft save, and publish | Extend it instead of building an unrelated editor |
| Publishing | `state.rs::publish_built()` allocates a new scene ID; `install_scene()` retains/clamps the current tokens and resizes exploration | Explicitly distinguish updating a scene from creating a new one; do not let publishing accidentally move the current encounter |
| Campaign cast | `campaign/CampaignPanel.svelte` can place a character as a token, currently at scene center | Add artwork selection and intentional placement with a ghost preview |
| Persistence | `server/src/storage.rs` accepts save version 1; unsupported versions are backed up and start a fresh table | Write an explicit migration. Merely incrementing the save version is not acceptable |
| Scene history | Active scenes, scene shelves, campaign shelves, character records, drafts, and saved undo/strike state exist | Migrate the whole save graph, not just the currently open map |
| Fog caching | Standees rebuild using explored-state changes; decals' cache key omits several changing properties, including fog | New rendering must have explicit dirty/revision tracking and test visibility changes without scene reload |
| Development serving | `client/vite.config.ts` already proxies `/packs` as well as `/api` and `/ws` | Preserve both development and built-server asset loading |
| UI instructions | `.cursor/rules/type-size.mdc` requires readable app text of at least 14px | New inspector/library controls must follow that rule |

**Assessment:** setting the camera is a relatively small change. The substantial work is making the editor, cutout depth, cast bindings, picking, movement, fog, and persistence all agree. This does not require a full-3D rewrite.

The generated camera-study sheets are **art-direction references only**. They are not calibrated geometry, verified seamless tiles, or production-ready asset specifications. The mathematical profile and executable tests in this plan are the technical authority.

---

## 3. The scene-building experience to deliver

### 3.1 Import assets

The GM opens **Assets**, chooses **Import pack**, and supplies a ZIP containing `pack.json` plus relative image files. Longtable validates the pack and shows Tiles, Objects, and Cast with thumbnails and useful metadata. A host-directory installation route may remain for development, but normal use must not require editing source files or restarting the server.

An invalid pack produces errors such as “`cast/bandit/north.png` is missing” or “tree pivot is outside its image,” not an empty catalog. One bad pack does not disable other packs.

### 3.2 Make a scene

The GM chooses a name and dimensions. The camera is already fixed; there is no projection question. The scene opens on a diagnostic/default ground material. Paint grass, dirt, cobbles, or ruin floor using one-cell or larger brushes. Add ground overlays such as leaves or puddles from the Tiles category.

Raise a selected area to tier 1 or tier 2. The editor builds vertical cliff/foundation faces automatically. Place stairs to connect adjacent tiers. Movement availability and invalid stair endpoints are visible in the editor.

Place Objects: trees and barrels use a footprint preview; walls and doors snap to logical grid edges. The art may overhang the footprint. Moving a tree must not turn its whole canopy into a solid ground obstacle.

Assign a Cast asset to a PC, NPC, or adversary in Campaign/character UI, then choose **Place** and click a valid surface. Show a placement ghost; do not silently drop every character at the center. Repeated adversaries can share artwork without sharing token IDs or game statistics.

### 3.3 Publish and play

Publishing a new scene makes a new scene. Publishing edits to an existing scene preserves that scene's identity, valid cast placements, door states, and exploration where appropriate. Invalid placements caused by terrain edits are listed for repair before publishing; do not silently teleport or delete cast.

Players see camera-matched cast standing on the correct surface. Trees, walls, barrels, and cliff faces obscure entities behind them. A permitted cast entity hidden by foreground scenery receives its own silhouette outline. Unknown enemies do not appear through walls or fog.

Switch scenes and campaigns, reconnect another client, restart the server: all scene layouts, asset references, cast appearance, facing, levels, and relevant states survive.

### 3.4 Required editor organization

Use the existing dock/panel system. Present **Tiles / Objects / Cast** as the main asset tabs, with search, pack filter, thumbnail, and selected-asset inspector. Tools such as terrain height, erase, select, and stairs are editing actions, not extra scene modes.

The same renderer draws Edit and Play. Optional editor-only overlays show footprints, blockers, stairs, anchors, and tiers; they do not change projection.

---

## 4. Coordinate and camera contract

### 4.1 Preserve world coordinates

Keep logical map `x, y`. In Three.js use:

```text
world.X = map.x
world.Y = height above base ground
world.Z = map.y
north = -world.Z
south = +world.Z
east  = +world.X
west  = -world.X
```

All positions, collision, distances, and saves use world/logical coordinates. Never store projected diamond screen positions as scene state.

The single camera profile is:

```text
profile ID:  dimetric-30-v1
projection:  orthographic
elevation:   30 degrees above the horizontal ground
azimuth:     45 degrees in the EXISTING projection.ts basis convention
```

The existing basis convention is important. It puts the camera on the **south-west** side of the map when azimuth is 45°. Do not silently use a different 45° convention from a modeling package.

Expected basis, rounded here for explanation, not for implementation:

```text
forward = ( 0.612372436, -0.500000000, -0.612372436 )
up      = ( 0.353553391,  0.866025404, -0.353553391 )
right   = ( 0.707106781,  0.000000000,  0.707106781 )

camera.position = focus - forward * pull
camera.up       = up
camera.lookAt(focus)
```

Use computed double-precision constants/functions in code. With screen coordinates increasing downwards:

```text
screenU = originU + S * dot(worldPoint - focus, right)
screenV = originV - S * dot(worldPoint - focus, up)
```

`S` is the current screen pixels per world unit. Pan/zoom may change origin and scale; they never change the basis.

### 4.2 What 128 pixels means

**Do not confuse source resolution, sprite scale, and projected cell size.**

* Ground source: 128 × 128 pixels over one square world cell.
* Sprite authoring scale: default 128 image pixels per screen-aligned world unit at the asset reference view; record it explicitly as `pixelsPerWorldUnit`.
* Camera zoom is independent of either file size.

At reference `S = 128`, a single square cell projects to a diamond approximately **181.02 pixels wide and 90.51 pixels high**. Its width:height ratio is exactly 2:1. A zoom with `S = 128 / sqrt(2)` instead makes a 128 × 64 screen diamond. Both use the same 128 × 128 source image.

No code or documentation should claim that a 128 × 128 source must display as 128 × 128 on an angled map.

A 128 × 256 cast frame is an image canvas, not a two-cell ground footprint and not a promise that every character is two world units tall. The pivot and render scale determine placement. Different creatures can use different canvases without changing the camera.

### 4.3 Replace view-specific assumptions

Update camera placement, pan, cursor-centered zoom, fit, center-on-cast, screen labels, placement ghosts, grids, measurement lines, and all artwork placement together. `fitMap()` should project the scene's 3D bounds, including elevation and tall object/cast bounds, into the visible viewport after dock insets.

For elevated picking, raycast valid terrain top surfaces/stair surfaces. A ray against the old `Y=0` plane is not sufficient. Reject side faces as walking destinations. The GM's height-paint tool may deliberately pick the logical top surface under the brush, with a clear highlight.

The fixed view does not mean the character cannot face north, east, south, or west. World facing changes the selected sprite, not the camera.

### 4.4 Required projection tests

Verify orthonormal basis; 2:1 projected square-cell ratio; equal ground-axis screen lengths; project → pick round trips at each tier; consistent panning across both map axes; cursor-centered zoom; framing with docks open; and no exposed message, save field, or UI control that can switch the scene camera profile.

---

## 5. Asset contract and authoring requirements

### 5.1 Versioning and identity

Introduce asset manifest `schemaVersion: 2` and `visualProfile: "dimetric-30-v1"`. Current manifests are versionless legacy inputs, not implicitly valid v2 packs.

Use globally unambiguous references such as `pack-id/asset-id`. For new packs, require IDs to be unique **across all three categories**, or use a typed reference structure everywhere. The simplest choice is pack-wide unique IDs; do not retain ambiguous IDs that collide between an object and tile.

Save explicit pack/content revision information with scenes. Do not overwrite a referenced pack with unrelated bytes under the same ID. For the first release it is sufficient to treat imported pack IDs as immutable and require a new ID for changed content; a sophisticated multi-version package manager is unnecessary.

Keep source images separate from thumbnails and any generated runtime atlases. Store original files so future validation/repacking is deterministic.

### 5.2 Tiles

Tile records include ID, name, category (`ground`, `overlay`, or material used for terrain sides), file(s), source size, coverage, filtering, and transition policy.

For standard ground tiles, require 128 × 128 PNGs, top-down, one-cell coverage. Opaque ground and alpha ground overlays are different uses. Larger object canvases must not be rejected just because the ground standard is 128px.

Support multiple terrain variants deterministically using scene seed + cell coordinate + asset ID, or a saved variant override. Variants must not change on reload, between clients, or when a neighboring cell is edited. Keep straight floors/cobbles unwarped. Optional organic edge blending must operate on transition masks, not distort the geometry or UVs of rigid paving.

Do not promise seamlessness merely because dimensions pass. The asset inspector must include a 3 × 3 repeat preview and mixed-terrain seam preview; artists must approve the joins. A texture with roots cut at its edges can have the correct dimensions and still tile badly.

### 5.3 Objects

Objects need independent definitions for:

* **Artwork:** RGBA file or render layers, actual canvas sizes, pixel pivots, render scale, and camera profile.
* **Placement/support:** local origin, ground footprint, placement type (`cell`, `edge`, or connection), and allowed terrain support.
* **Movement:** blocking footprint/polygon or edge, independently enabled.
* **Vision:** finite-height blocking volume(s), independently enabled.
* **Visual occlusion:** cutout/render-layer geometry, independently enabled.
* **Behavior:** static object, wall, door, or stairs; state variants where relevant.

A tree can have a large illustrated crown and a small trunk blocker. A barrel may block walking and obscure a character's legs without blocking sight at eye height. A wall can block both movement and sight. A ground stain does neither.

Standardize pivots as `[u, v]` pixel coordinates measured from the image's top-left. The pivot maps to a documented local world point—normally the object's root/support origin. Do not reuse the previous front-left footprint anchor implicitly. The importer preview must show this point and the footprint at the fixed camera.

Objects spanning several cells must rest on compatible terrain heights. Reject a table halfway inside a cliff unless a purpose-built support rule is present. Overhanging foliage is fine; that is artwork, not support.

For walls, the old “front/side” images were created for the old north-facing view. The new two grid-axis variants must both match the diagonal 30° view. A PNG cannot be turned into a valid opposite wall view just by rotating it in image space. State this in the asset guide.

### 5.4 Cast

Store cast artwork separately from character portrait art. A standard cast asset has:

```text
four facing frames: north, east, south, west
optional portrait/thumbnail
per-frame image size and foot/root pivot
explicit render scale
optional sockets, reserved for future equipment attachment
```

Facing is world-relative: north is -Z, east +X, south +Z, west -X. **These are camera-matched three-quarter views at the fixed diagonal camera, not generic eye-level front/back/profile portraits.** A high-quality character turntable sheet at the wrong elevation is still incompatible.

Use the same character proportions and grounding point in all frames. Do not auto-mirror asymmetric equipment or lighting. A standard humanoid can use a 128 × 256 canvas with transparent padding; it is a recommended canvas, not an enforced world height. Larger creatures may use larger files with explicit scale and footprint rules.

For application reliability, missing cast art uses a neutral upright diagnostic silhouette in the **same renderer**, with a clear missing-asset warning. The fallback is not a second disc-token scene mode.

Appearance resolution should be explicit:

```text
placement override → linked character's castAssetId → diagnostic fallback
```

Resolve the public render data on the server so another player can draw an authorized adversary without receiving its private character sheet. A sheet's portrait URL must not override the cast asset.

### 5.5 Image safety and reproducibility

Require actual decoded dimensions, valid color/alpha format, finite numeric fields, legal pivots, safe relative paths, existing referenced files, supported profile, and allowed image-size budgets. Reject unknown required behaviors instead of silently ignoring them.

Use sRGB for color images and linear/no-color-space for masks, IDs, and depth. Apply a consistent alpha cutoff to the color, depth, ID, and outline passes. It is not enough to alpha-test only the visible color material.

The standard path needs neither depth-map authoring nor 3D models. Multi-layer sprites handle modest object depth. More sophisticated per-pixel depth assets may be added later, but must not be a prerequisite for a tree or barrel pack.

---
## 6. Scene model and ownership

Introduce a versioned canonical scene layout. Keep compact row-major terrain storage where useful; a new UI vocabulary does not require renaming every existing function.

The model must represent:

```text
Scene
  id, name, schemaVersion, revision, visualProfile
  dimensions, deterministicSeed, referencedPackRevisions
  layout
    terrain palette + one terrain index per cell
    optional saved variant/rotation overrides
    one elevation byte per cell: 0, 1, or 2
    ground overlays
    placed objects, including wall/door subtypes
    stair connections
    existing light markers, with their existing gameplay meaning preserved
  cast placements                       [NOT in public layout]
  current door/object states
  exploration/visibility state
```

A logical object can use an edge anchor for a wall/door, a cell/ground anchor for a prop, or paired endpoints for a stair connection. All remain Objects in the user interface. Keep the useful distinction in types; do not force edge walls into a generic center-snapped prop model.

A character's `castAssetId` is a default appearance. A placed cast entity has a stable ID, optional `characterId`, optional appearance override, position, facing, ownership/hidden/vision fields, and explicit gameplay footprint. Its world height is derived from authoritative terrain support, not accepted as an arbitrary client-supplied number.

For footprints, define one consistent center/origin conversion. Existing 1 × 1 tokens keep half-integer cell-center positions. An even-sized footprint may have an integer center. Add helper functions and tests instead of scattering `floor(x) + 0.5` throughout the code. Require same-tier support under the whole occupied footprint; the minimum acceptance set includes 1 × 1 cast and one 2 × 2 diagnostic cast.

**Visual size must not silently change gameplay size.** Assigning a tall portrait/sprite does not turn a PC into a two-cell creature. Changes to gameplay footprint are explicit and server-authorized.

### Private data separation

The current server sends the built map layout to players. Therefore do **not** put unrevealed adversary positions, hidden flags, private character sheets, or secret cast metadata inside `SceneView.layout`.

GM draft cast placements, when supported before publication, must be separate GM-only data. Publishing materializes/reconciles them into the existing server-side token/cast collection. Player snapshots receive only permission-filtered render records.

Generic library artwork being downloadable is not the same as revealing that a particular enemy exists at a particular map position. Preserve that distinction. Do not claim the full scenery layout is secret while transmitting it to the browser; this plan's strict non-disclosure requirement concerns concealed cast and private state.

---

## 7. Rendering objects and cast without a full-3D art pipeline

### 7.1 Use one hybrid 2.5D pipeline

Use actual world coordinates, lightweight terrain geometry, alpha-cutout scenery, and depth testing. Do not implement a second canvas-only compositor or retain prerendered-scene switching as an alternative runtime path.

For grounded cutouts, use **world-upright, horizontally view-aligned quads** with their root on the support surface. Their horizontal direction is the camera's `right`; their vertical direction is world +Y. For camera-matched source art, compensate for projection in the quad's vertical dimension:

```text
u,v                  = image coordinates relative to its pixel pivot
horizontal world     = u / pixelsPerWorldUnit
vertical world       = -v / (pixelsPerWorldUnit * cos(30°))
world vertex         = root + right * horizontal world + worldUp * vertical world
```

This preserves the image's intended screen proportions while giving upper pixels a meaningful vertical/depth relationship. Do not blindly copy the tilted camera quaternion to every quad: that puts the card parallel to the screen and makes its whole face share one depth.

This is still a **2.5D approximation**, not recovered 3D anatomy or a volumetric tree. It is adequate for rooted cutouts and modest scenery when the following limits are respected.

### 7.2 Layered objects and ground bases

Give multi-part objects optional render layers with their own local ground anchors, height offsets, pivots, and alpha images. A tree may have a ground-root overlay plus trunk/canopy cutout layers. A broad table or long wall should be represented by suitable slices/modules rather than one enormous incorrectly sorted card.

Separate a flat ground shadow/root base from upright artwork. Opaque pixels below an upright root pivot can otherwise project below the terrain and clip incorrectly. The importer preview must reveal this; reject unusable pivots or require a ground layer rather than inventing a depth hack.

Walls should use connected one-cell modules with both grid-axis images, corner/cap rules, and real support heights. Cliff faces are actual geometry, not always-on-top sprites. Large object layers must pass front/back traversal tests before the asset is marked compatible.

Do not use an invisible solid rectangular proxy as the sole visual occluder for a sparse tree; that would hide cast through transparent gaps. Visual depth follows the artwork's alpha. Physical and sight volumes are separate and can be simpler than the art.

### 7.3 Depth and transparency rules

* Default cutouts: depth test on, depth write on, alpha cutoff shared across passes; render as cutouts rather than relying on general transparent-object sorting.
* Ground decals/shadows: depth write off, small controlled surface offset; never occlude cast by themselves.
* Fully translucent effects: separate late pass, non-blocking by default; not required for the first asset-ready release.
* Do not globally draw all objects after all cast. That is not a substitute for depth.
* Any layer depth offsets must be authored in world coordinates and documented, not arbitrary per-camera magic numbers.
* Avoid depth conflicts between cast feet, base marker, terrain, and selection outlines. Use consistent small offsets, not disabling depth tests globally.

### 7.4 Fog in the same pipeline

A flat fog plane above `Y=0` cannot correctly cover raised surfaces or tall art. Apply the fog state in scene materials/compositing using world positions or documented support anchors. Terrain and cliff materials sample the relevant world X/Z location. Cutout scenery uses its support/layer relationship; split very broad scenery into layers where necessary.

Cast existence and visibility are filtered by the server first. The renderer must not reconstruct missing tokens from cached geometry or character lists. Clear removed or newly hidden cast from all color, depth, ID, outline, hover, and label caches immediately.

Use an **effective viewer** helper for GM versus “see as player.” The current built-map `role === 'gm'` show-all shortcut must not bypass player-preview visibility.

---

## 8. Occlusion outline contract

### 8.1 Required behavior

A cast entity behind a tree, wall, barrel, or cliff can be camera-obscured even though a character on the map is allowed to see it. In that case draw a restrained outline following the cast artwork's alpha silhouette. Visible body portions remain normally rendered.

If the game server says the entity is hidden or outside the viewer's permitted knowledge, **there is no body, outline, name, marker, hit target, or hover tooltip**. An outline is a readability feature, not a see-through-walls game ability.

Default: all server-permitted cast may receive an occluded outline, with stronger emphasis for the selected/controlled cast. The implementation must not require selecting an invisible rectangle before the hidden cast can be found.

### 8.2 Suggested implementation

Use a bounded screen-space mask/depth pipeline:

1. Obtain scenery/terrain depth, including alpha-cutout holes, excluding cast and non-blocking decals.
2. Render the **authorized cast only** into an offscreen mask/ID/depth target without scenery occluding them. Respect the sprite alpha, pivot, facing, and terrain height.
3. For each cast pixel, compare cast depth against scenery depth to determine camera occlusion with a small, documented depth tolerance.
4. Compute the cast silhouette boundary from the full alpha/ID mask; apply the hidden-outline color where that boundary is obscured. Do not simply outline the hidden region itself, which would draw a false contour across the middle of a body where a barrel cuts across it.
5. Composite into the fog-aware final scene. Keep the stroke approximately 1–2 CSS pixels at ordinary zoom, accounting for device pixel ratio.

Handle adjacent/overlapping cast with IDs so a union mask does not fuse every silhouette into one blob. A nearest-cast-per-pixel policy is acceptable for unselected overlapping cast if documented; selected-cast emphasis must still be usable. No full-screen render target per cast in a large encounter.

Three.js offers outline/depth facilities, but do not assume a stock outline pass automatically respects arbitrary RGBA sprite alpha or custom cutout materials. Verify that explicitly. A custom sprite-mask pass is preferable to an outline around the image quad.

### 8.3 Picking and selection

A visible occluded outline must be selectable. Use the authorized cast ID/mask or alpha-aware raycast candidates. Transparent PNG padding is not a valid hit. Do not perform blocking GPU pixel readback every frame; restrict it to clicks or use a cached CPU alpha mask with proper projection.

Normal scenery objects do not automatically need x-ray outlines. The required x-ray feature is for Cast; ordinary object selection can have a separate editor outline.

### 8.4 Acceptance matrix

Test fully visible, partly obscured, fully camera-obscured, tree-alpha-hole, foreground/background swap, different tiers, two overlapping cast, missing sprite, hidden-by-GM, outside current sight, unowned enemy, own cast, GM view, and GM “see as player.” Test at multiple zooms and device pixel ratios.

The critical security test is a hidden adversary directly behind the outlined friendly cast: it must never enter any outline/ID buffer or reveal its position through hover.

---

## 9. Bounded elevation, movement, and sight

### 9.1 Three tiers, not stacked floors

Use a heightfield: one walkable horizontal surface at each X/Z cell, with levels `0`, `1`, and `2` corresponding to world heights `0`, `1`, and `2`. The third tier is two units above ground, not “three upper floors.”

This supports low ground, raised ruins, platforms, and high ridges. It does not support two walkable rooms at the same X/Z, an underpass beneath a bridge, or independently occupiable rooftops over ground. Do not imply those work.

Build exposed vertical faces from neighboring level differences and map boundaries. Give them a side material. Do not leave holes around raised cells or draw the old floor grid through solid raised terrain.

### 9.2 Stairs and navigation

Stairs are connection objects with explicit lower and upper endpoints. Start with cardinal connections between adjacent tiers. Validate endpoint heights, free space, orientation, and sufficient width for the cast footprint. A transition from tier 0 to tier 2 needs two valid steps, not a hidden teleport.

The initial game model can treat a stair as a connection between discrete surface cells; no continuous slope physics is necessary. Its visible geometry must connect those surfaces. Picking a staircase should resolve a documented destination, not an arbitrary world height.

Server-side movement must:

* derive the destination surface and height;
* validate the entire traversed route, not only endpoint height;
* reject cliff crossings without a connection;
* honor wall/door and object blockers, same-tier support, footprint, and no diagonal corner cutting;
* use deterministic, bounded pathfinding for drag-to-destination, with a visible path preview and authoritative server confirmation;
* handle unreachable destinations with a readable reason;
* keep deliberate GM relocation as an explicit override, not something players can achieve by forging a height.

Use existing system distance conventions where applicable. Do not introduce a new movement allowance or action cost silently. Routing correctness is required; a universal combat movement-budget system is not part of this goal.

### 9.3 Height-aware visibility

The current 2D walls/polygon implementation treats blockers as infinitely high. It is not sufficient to move the art upward while keeping that calculation unchanged.

Introduce finite vertical spans for sight blockers. For example, a blocker has a ground footprint or edge prism plus `baseHeight` and `height`, independently of its movement flag. Terrain itself is solid below its surface. Closed doors contribute their sight volume; opening a door removes it.

Use server-side line-of-sight tests from observer eye position to the target point. A grid traversal/spatial index can find intersected terrain cells and blocker volumes; compare their vertical intervals with the height of the ray over the intersected horizontal interval. Compare the interval, not merely a blocker midpoint. Treat segment endpoints and self-support consistently so a character does not occlude its own vision.

For the first version, explicitly document and test standard eye and target heights, for example an eye 1.0 world unit above the observer's support and a cast visibility sample 0.8 above the target's support. These are tabletop visibility conventions, not anatomical simulation. Keep them in one place. Object/cast metadata may later specialize them through validated gameplay fields.

Use the same authoritative visibility service for token filtering and fog generation. Replace or adapt the old polygon transport to a compact visible-cell/subcell mask suitable for elevated terrain; preserve explored memory. Existing explored cells can remain world-X/Z based because only one surface exists per cell.

A high-ground observer may see over a low blocker if the ray actually clears it. A tall wall can still block. An elevated target can be visible while the floor below it is not: token visibility should use the explicit target test, not only the floor-cell bit.

Preserve existing ownership/shared-sight and exploration-sharing behavior unless a separate product change is requested. Do not rewrite party-vision policy as an incidental rendering change.

### 9.4 Combat integration

Expose the selected/target tier and elevation difference. Make height available to rule-system adapters, but keep existing Daggerheart/other system mechanics intact. Do not invent an automatic damage, advantage, or range bonus. GM decisions and existing rules remain authoritative.

Physical route restrictions, finite-height visibility, and a useful elevation indicator already make terrain tactically meaningful without forcing new game rules.

---
## 10. Implementation goals and acceptance gates

The goals below are ordered vertical slices. Finish each with a small implementation report: files changed, tests run, visible result, remaining limitations. Elevation is not releasable after G7 alone; G8 completes its visibility semantics.

### G0 — Establish a protected baseline and a rendering test harness

**Outcome:** changes can be evaluated against a known baseline without risking user saves.

**Work**

- Read repository instructions, including `.cursor/rules/changelog.mdc` and the 14px UI rule.
- Record HEAD, package/lockfile versions, current tests, and known existing failures. Do not upgrade the framework or Three.js as part of this task unless a demonstrated blocker requires it.
- Back up test copies of data, never the user's only working data. Introduce migration fixtures containing an active scene, another scene, another campaign, linked and unlinked tokens, exploration, doors, and missing-art cases.
- Add browser integration/render tests capable of detecting WebGL/shader console errors and taking repeatable screenshots. Existing Vitest node tests cannot establish that the scene actually renders.
- Create an asymmetric diagnostic fixture: labeled X/Z directions, one raised marker, known cell centers, a tall opaque cutout with transparent holes, and a cast-shaped alpha sprite. Use tiny generated/owned assets, not cropped concept posters.

**Acceptance**

- Baseline results are written down, including failures that predate the change.
- A headless/browser smoke test loads a scene with no shader compilation errors.
- Fixture files and visual baselines are deterministic; no external art service is needed.
- No tests use or overwrite the user's actual `data/state.json`.

### G1 — One fixed camera and safe canonical scene scaffolding

**Depends on:** G0.

**Outcome:** every playable test scene uses the new projection, with correct navigation and framing.

**Work**

- Implement a single camera profile module around existing `projection.ts` mathematics; replace old `ELEVATION_SIN`, `GROUND_DEPTH`, `HEIGHT_SCALE`, and standee basis assumptions in scene rendering.
- Adapt fit, pan, zoom, centering, world-to-screen labels, grid lines, measurement, and edit ghosts. Use projected 3D bounds.
- Introduce canonical scene/layout version fields and level-zero defaults. Establish explicit save-version migration scaffolding before changing storage format.
- Make blank/new scenes canonical native layouts. Keep old save parsing in migration code only; do not silently interpret a baked angled map as a top-down texture.
- Disable/remove camera orientation controls and alternate presentation selection from playable scene UI. Legacy data that cannot yet convert is preserved and flagged, not rendered using the old camera.
- Leave game rules, character data, dice-tray rendering, and application Edit/Play/Design workflows intact.

**Acceptance**

- Projection tests from section 4 pass, including a 2:1 diamond.
- A screenshot of the diagnostic axes confirms the existing azimuth convention, not a mirrored/rotated replacement.
- Pan/zoom/fit work with all docks open; world picking matches the painted grid.
- New scene and campaign entry points use the same renderer/profile.
- There is no player/GM control or accepted state message for scene-camera rotation/tilt.

### G2 — Versioned asset contract, validated import, and the asset browser

**Depends on:** G1.

**Outcome:** a GM can load a pack and immediately find its Tiles, Objects, and Cast without code changes.

**Work**

- Define Rust types and machine-readable validation for the v2 contract; export shared types for the client. Add schemas/examples to repository documentation.
- Implement a GM-only import endpoint, with a client file picker, validation report, thumbnails, category tabs, search, and pack filters.
- Stage and validate imports outside the served directory, then install atomically. Reject absolute/traversal/backslash-escape paths, symlinks, duplicate/case-colliding paths, remote file URLs, executable content, unsupported image types, and decompression bombs.
- Use explicit limits. Initial proposed defaults: 100 MiB compressed, 512 MiB expanded, 2,000 files, and 2,048px maximum object/cast image side; all are configurable server constants with tests. Standard ground images remain 128 × 128. Never allocate from untrusted dimensions without a decoded-size/pixel-budget check.
- Verify every required file and actual decoded size; isolate invalid packs from valid packs. Report exact file/field failures.
- Refresh the server catalog and notify clients of catalog revision changes. Replace the client's permanently cached single-load behavior. Keep catalog mutation/IO out of the table-state mutex where possible.
- Treat imported IDs as immutable for this release. Identical re-import can be idempotent; conflicting bytes under an existing ID fail clearly. Support new IDs without restarting.
- Keep imported source files and compute content hashes. Do not silently replace collision metadata in a running scene.
- Provide an asset inspector showing fixed-camera preview, alpha, pivot, footprint, render bounds, blockers, facing frames, and tile repeat preview.

**Acceptance**

- A complete pack appears in all appropriate tabs after one import; a second connected browser sees the new catalog.
- Invalid imports install nothing and do not remove existing assets.
- Unauthorized import, missing frames, wrong dimensions, unsafe archive paths, repeated IDs, and unknown profiles have automated tests.
- Missing optional portrait/thumbnail does not make a valid cast unusable.
- Development Vite and production Rust serving both load pack files correctly.

### G3 — Correct one-cell terrain and a usable tile editor

**Depends on:** G2.

**Outcome:** the GM paints real source tiles into a persistent fixed-camera scene.

**Work**

- Replace the implicit four-cell texture repeat with explicit one-cell coverage for the standard contract.
- Implement deterministic variant choice and saved overrides; do not load only the first variant.
- Keep UVs stable across cells/chunks. Do not warp cobbles or rigid stone flooring. Make organic blending optional and material-aware.
- Implement ground overlays under Tiles; preserve useful existing decals/theme brushes while normalizing their definitions.
- Batch/chunk terrain rather than creating a material and draw call for each cell. Design this path to accept tier heights in G7 without replacing it again.
- Ensure image completion/error marks the renderer dirty. A successfully loaded texture must appear while the camera is idle.
- Preserve stroke-based undo/redo and autosave; make save errors visible and stop publishing when the save failed.

**Acceptance**

- A 128 × 128 calibration tile occupies exactly one cell, not four; asymmetric marks demonstrate correct UV orientation.
- The same scene/seed produces the same variants on another client and after restart.
- Tiles and overlays update without requiring a pan/zoom to repaint.
- Tile corners, seams, overlay alpha, mixed-terrain transitions, and undo/redo are covered by visual tests.
- A published flat scene round-trips through save/load unchanged.

### G4 — Objects, wall kits, doors, and independent blockers

**Depends on:** G3.

**Outcome:** objects can be placed meaningfully and behave separately from their artwork.

**Work**

- Introduce the object renderer and shared cutout/material factory described in section 7. Replace old hard-coded standee orientation and front-left anchoring.
- Implement footprint previews, selection, movement, deletion, and inspector controls. Allow foliage overhang without enlarging the trunk collider.
- Support render layers and flat ground bases. Disable arbitrary image rotation/mirroring for upright art unless a validated matching variant exists.
- Expose wall/door assets as Object subtypes while keeping edge snapping, IDs, and authoritative door state.
- Implement both logical edge orientations, corner/end pieces, and matching door opening/state images. Validate door references and heights.
- Compile independent movement and finite-height sight blockers. At this stage flat-ground visibility may still use the existing adapter, but preserve real heights for G8.
- Server-validate IDs, categories, footprints, support, bounds, duplicate instances, door occupancy, and blocking geometry; do not rely solely on editor checks.
- Make missing artwork visibly diagnosable without silently removing physical blockers. A scene should retain validated gameplay geometry or refuse activation if it cannot reconstruct that geometry safely.

**Acceptance**

- Tree, barrel, table, wall, and door all have correct pivots and support at the fixed camera.
- Cast-shaped test cutouts pass in front of and behind scenery correctly; transparent canopy gaps do not act as opaque rectangles.
- A barrel can stop movement without acting as an infinitely high sight wall.
- Both wall orientations connect, and a door opening updates art and blockers together.
- Duplicate IDs, off-map footprints, malformed volumes, and unsupported object orientations are rejected server-side.

### G5 — Cast assets on PCs, NPCs, and adversaries

**Depends on:** G4.

**Outcome:** actual character sprites replace portrait/disc stand-ins throughout normal scene play.

**Work**

- Add character default appearance and placed-cast appearance/facing fields, with backwards-compatible migration defaults. Preserve IDs, ownership, private sheet data, and existing game-system behavior.
- Implement appearance resolution, server-side public render records, and a Cast renderer using root pivots and correct support heights.
- Integrate asset selection into Campaign/character/adversary workflows and the placed-token inspector. Preserve portraits for sheets/UI.
- Add a click-to-place ghost instead of unconditional scene-center placement. Support repeated adversaries with independent instance state and shared asset references.
- For pre-publication cast placement, keep the draft records GM-only and outside the public layout; reconcile them on publish.
- Add world-relative facing controls and deterministic facing after movement. A static camera stays static. On diagonal moves, use a documented cardinal selection rule; no flickering or image flipping.
- Preserve gameplay footprint independently of visual scale. Test one 2 × 2 diagnostic cast as well as normal 1 × 1 cast.
- Use a named upright diagnostic fallback for old/missing assets; remove the ordinary-map disc versus imported-map portrait-card branch.

**Acceptance**

- A PC and adversary can each be assigned artwork, placed, moved, faced, saved, and reloaded.
- Changing a character default updates linked placements unless they have an explicit override.
- Different adversaries can use the same asset without sharing HP, IDs, ownership, or facing.
- Another player can see the authorized sprite without receiving the adversary's private character sheet.
- Portraits, dice, attack/resource actions, sheet creation, and existing ownership permissions still work.

### G6 — Cast-shaped occlusion outlines and reliable selection

**Depends on:** G5.

**Outcome:** obscured cast remains readable without revealing hidden entities.

**Work**

- Implement the alpha-aware world-depth and authorized-cast mask/ID/depth pipeline from section 8.
- Share alpha cutoff, texture UVs, sprite transforms, and material behavior across all passes.
- Add restrained hidden outlines, stronger selected-cast emphasis, alpha-aware picking, and proper label visibility.
- Apply effective-viewer semantics, including GM “see as player.” Remove unauthorized entities from every buffer/cache on the same state update.
- Dispose/resize targets correctly and invalidate them on camera, catalog, object, door, terrain, cast, and visibility revisions. Reuse cached static depth when nothing affecting it changed.

**Acceptance**

- The full matrix in section 8 passes in both automated screenshots and a two-client session.
- No rectangular outline appears around transparent sprite padding.
- A partly hidden cast shows an appropriate silhouette, not a false line across its body at the obstacle edge.
- Clicking an outlined hidden body selects the correct permitted cast, not the foreground tree or an absent enemy.
- Hidden adversary data never reaches the mask/ID/outline input set.

### G7 — Three-tier terrain, stairs, and authoritative movement

**Depends on:** G6.

**Outcome:** elevated scene geometry, placement, and walking work together.

**Work**

- Add/persist/validate one level byte per cell and a terrain-height brush for tiers 0/1/2.
- Render top surfaces and exposed side faces in the same terrain renderer. Lift ground overlays, cast, objects, selection bases, and labels appropriately.
- Introduce stairs as validated connection objects. Start with cardinal, one-tier connections.
- Replace `Y=0` pointer assumptions with surface picking. Show tier and chosen destination in the placement/movement preview.
- Implement bounded server navigation, support/footprint validation, cliff restrictions, no corner-cutting, and explicit GM relocation override.
- Save/reload draft stairs, level edits, objects, and placements. Changing terrain under occupied cells requires repair/confirmation rather than silent relocation.

**Acceptance**

- A fixture contains all three tiers and two stair connections; a player can reach the top by the valid route.
- A direct forged move up the cliff fails; a valid path using stairs succeeds.
- Cast does not float, sink, or change footprint on raised terrain; 2 × 2 cast cannot straddle unsupported heights.
- Side faces do not become valid click-to-walk destinations.
- Stairs and tier changes survive undo/redo, publish, reconnect, and restart.
- Mark elevation as not release-complete until G8 passes.

### G8 — Height-aware sight, fog, and permission consistency

**Depends on:** G7.

**Outcome:** higher ground affects what can be seen without creating information leaks.

**Work**

- Implement finite-height line-of-sight and terrain-solid intersections with spatial indexing and cached viewer computations.
- Derive visible masks and token permissions from the same authoritative service, with appropriate floor versus cast sample points.
- Adapt fog transport/rendering to elevated geometry; preserve existing exploration policy and data migration.
- Make closed/open doors and blocker changes invalidate affected visibility.
- Expose elevation difference in the selection/target UI without introducing unrequested combat bonuses.

**Acceptance**

- A high observer sees over a low obstacle only when the ray clears it; a sufficiently tall wall still blocks.
- A target on a higher platform can be visible independently of the terrain beneath it.
- Fog shades raised tops and cliff faces, not just the ground plane.
- Server withholding, client disappearance, outlines, labels, hover, and “see as player” agree after every visibility change.
- Existing same-tier sight/ownership cases remain correct.
- Visibility recomputation is benchmarked; it does not run from scratch for every idle frame or unrelated dice event.

### G9 — Unified scene workflow, complete migration, and legacy cleanup

**Depends on:** G8; migration scaffolding began in G1.

**Outcome:** the app has one coherent scene method and existing user data is not sacrificed.

**Work**

- Finish the explicit migration strategy in section 11 across active scenes, shelves, campaigns, drafts, cast, and saved ancillary state.
- Make scene create/edit/duplicate/publish/switch/delete flows explicit. Add scene/draft revision checks so stale autosaves cannot overwrite a newer scene.
- Updating an existing scene preserves ID and valid cast/state. Creating or duplicating a scene uses new IDs intentionally, not accidental current-token carryover.
- Remove runtime dependencies on `.ltmap` presentation loading, EXR scene-depth maps, per-view camera selection, and Dungeon Alchemist import controls. Keep migration readers isolated where necessary.
- Replace outdated documentation and old asset formulas. Remove dead flags, old UI controls, unused generated message variants, and obsolete tests only after replacement coverage exists.
- Do not remove unrelated system documents, dice textures, campaign functionality, or ordinary source assets merely because they are old.

**Acceptance**

- All playable scenes use `dimetric-30-v1`; old presentation data cannot activate a hidden alternate renderer.
- Old saves and drafts migrate without lost campaign/character IDs or silent data resets.
- Unsupported legacy visual content is archived with an actionable report and, where possible, its gameplay layout is recovered into a new native scene.
- Editing a map with cast on it does not unexpectedly create a new scene, move the encounter, reset explored space, or reset a door.
- A missing referenced pack never silently erases collision or enables forbidden movement.

### G10 — Prove asset readiness and hand off the authoring guide

**Depends on:** G9.

**Outcome:** a user can supply art and build/play scenes without asking the coding agent to repair each pack.

**Work**

- Ship a self-contained diagnostic pack with actual files and a representative test scene. Suggested minimum: four ground tiles, one overlay, one cliff-side material, tree, barrel, table, wall kit, door kit, stair art, and two four-view cast assets. Diagnostic art should be readable but is not required to resemble production fantasy art.
- Write `docs/assets.md` or a replacement asset guide from the implemented contract—not from the earlier posters. Include exact image requirements, camera coordinates, pivots, visual/physics separation, actor facing, walls, stairs, pack examples, and import errors.
- Provide a pack template, JSON schema, repeat-preview/check tooling, and an asset validator command or endpoint that users can run before import.
- Add a GM tutorial: import pack → paint terrain → place objects → add stairs → assign cast → publish → verify as player.
- Run all automated tests, production build, browser render tests, migration fixtures, and a real two-client smoke test.
- Benchmark representative and supported-limit scenes. Record hardware/browser and measurements; do not assert unmeasured performance.

**Acceptance**

- A fresh checkout and empty data directory can import the diagnostic pack and construct the acceptance scene entirely through UI.
- A clean player browser joins and behaves correctly; a restart restores the same result.
- No TypeScript edits to generated bindings are required to make the build pass.
- The production artifact reports unsupported/missing assets clearly; no silent empty catalogs, stale geometry, or hidden state leaks remain.
- Documentation, schemas, runtime behavior, and actual example dimensions agree.
- The coding-agent final report distinguishes completed features, passed tests, measured performance, and any still-open limitations.

---
## 11. Migration and failure behavior

### 11.1 Never turn a migration problem into a fresh game

Implement explicit version detection and a version-1 → version-2 migration. Unknown future versions and failed migrations must stop destructive persistence and present a recovery error; they must not create an empty replacement save that autosave writes over the user's game.

Before migration, make a non-overwriting backup with timestamp/content identifier. Preserve original files. Validate the migrated model before atomic replacement. Produce a report listing converted scenes, unsupported visuals, missing packs, inferred defaults, and anything requiring GM review. Test rollback using the backup.

Migrate the active scene and cast, inactive scene shelves, campaign shelves, characters, drafts, exploration, and any saved undo state containing old structures. Where an old undo snapshot cannot be migrated safely, explicitly clear that undo history and report it rather than retaining an invalid snapshot. Preserve pending gameplay state when compatible.

### 11.2 Legacy inputs

| Legacy content | Required handling |
|---|---|
| Blank native map | Create a level-zero canonical layout with diagnostic/default tiles |
| Built map with current core definitions | Convert terrain, props, edge pieces, decals, and lights; level defaults to zero; adapt references and report art that belongs to the old camera |
| Portrait-only token | Keep portrait for UI; assign upright fallback until real cast art is chosen; preserve character linkage and statistics |
| Existing top-down bitmap map | Preserve original and logical geometry. A dedicated converter may recover it into the native pipeline; do not promise editable trees or walls extracted from image pixels |
| `.ltmap`/baked angled scene | Preserve/archive original presentation assets. Recover known logical data where safe into a native diagnostic scene, or flag rebuild required. Do not skew its baked color image and call that a correct 30° scene |
| Missing pack or image | Retain asset references and known validated collision/state; show a placeholder and diagnostics. Reject unsafe reconstruction rather than dropping blockers |
| Old wall with no height | Apply a documented finite migration default, e.g. 2 world units, and report the assumption. Do not pretend the original source provided a height |

“All scenes use one method” does not mean automatic recovery of 3D information from a baked picture. It means there is no legacy runtime camera mode. Scenes that need rebuilding remain preserved records until converted; they are not silently destroyed or silently rendered in another projection.

### 11.3 Publishing and concurrency

Add draft/scene revision checks. A failed or stale save must prevent publish. Distinguish **Publish new scene**, **Apply changes to this scene**, and **Duplicate scene** in intent, even if one context-aware button handles them.

When applying changes, retain valid token IDs, character links, positions, explored cells, and compatible door IDs/state. Recompute derived navigation/visibility atomically against the new scene revision. List incompatible cast/object/stair placements before activation. Require an explicit repair choice; no unnoticed automatic teleport.

Keep a validated gameplay snapshot or enough persisted resolved collision/support metadata to prevent a missing artwork pack from disabling movement barriers. Do not trust a serialized cache blindly: version and bind it to the layout/asset revision. If neither valid definitions nor trustworthy compiled data are available, refuse unsafe scene activation and provide recovery instructions.

---

## 12. Performance, robustness, and test strategy

### 12.1 Engineering constraints

- Preserve render-on-change behavior. An idle table should not rebuild meshes, recompute navigation/vision, or render multiple full-screen passes continuously.
- Use chunked terrain; cache shared textures/materials; consider instancing/atlasing repeated objects only after correct behavior exists. Avoid one material/draw call per ground cell.
- Separate static terrain/object depth invalidation from dynamic cast/outline updates. Rebuild only affected chunks/instances where practical.
- Track changes by explicit revisions and dirty flags rather than repeatedly serializing the whole object list into a cache key.
- Notify the renderer after asynchronous asset completion. Generation/version checks must prevent a late response from replacing newer or disposed artwork.
- Dispose geometries, materials, cached textures with correct ownership/reference counting, render targets, listeners, and temporary object URLs on scene changes/unmount.
- Keep ID/depth textures nearest-filtered and non-color-managed. Test render-target resize and browser device-pixel-ratio changes.
- Put bounded work and allocation limits on import, image decoding, pathfinding, map validation, and sight computation.
- Benchmark server visibility/pathfinding separately from browser FPS. Spatially index blockers and reuse unchanged vision results.
- Do not recreate every standee when one door opens or a single actor moves.

### 12.2 Representative acceptance scene

Create a deterministic **12 × 12 forest-ruin encounter** with all of the following:

1. Four correctly scaled ground materials and a transparent ground overlay.
2. An oak with a crown wider than its physical trunk footprint, a barrel, and a table.
3. Connected walls in both grid-axis directions and one working door.
4. Surface tiers 0, 1, and 2, exposed sides, and two valid stair links.
5. A linked PC, a visible adversary, a deliberately hidden adversary, and a second cast using shared artwork.
6. A cast behind the tree that the viewer is allowed to know about; its own outline must appear.
7. A secret cast behind an opaque wall; neither its outline nor hover target may appear.
8. A high-ground sight test involving one low blocker and one sufficiently tall wall.

Use the same fixture in GM, player, and GM-as-player tests. It is the actual proof that the features work together, not only separately.

### 12.3 Scale/performance tests

Benchmark at least a 50 × 50 scene with 500 objects and 50 cast, then exercise the current supported map-size ceiling of 200 × 200 without crashing or violating validation limits. Record reference hardware, browser, render resolution, asset memory, draw calls, frame times while moving/panning, and server action/visibility latency. These are benchmark scenarios, not claims that measurements have already been made.

Pick explicit performance budgets after the baseline is measured and record them. Do not make quality decisions from guessed FPS. At minimum, moving one cast should not rebuild all terrain or re-import/redecode every asset, and repeated scene switches should not grow GPU memory indefinitely.

### 12.4 Test commands and additional coverage

Run the repository's existing commands on the implementation checkout:

```bash
cargo test --manifest-path server/Cargo.toml
cd client
npm ci
npm run check
npm test
npm run build
```

Run new browser/render and migration tests with commands documented by the agent. Keep Rust-exported bindings synchronized; verify the build does not depend on manual generated-file edits.

Include malformed-message tests, unauthorized appearance/placement changes, missing assets, duplicate IDs, invalid levels, unsupported camera profile, oversized imports, save migration failure, stale publish, camera-hidden versus game-hidden cast, and scene switching during asynchronous texture load.

---

## 13. File-level implementation map

This map identifies where current code should be adapted. Proposed new files are suggestions, not files claimed to exist today.

| Existing area | Planned work |
|---|---|
| `client/src/lib/render/projection.ts`, `projection.test.ts`, `view.ts`, `view.test.ts` | Single camera basis, calibrated projection, replacement for old footprint-image formulas |
| `client/src/lib/render/engine.ts` | One render path, projected fit, elevated picking, pan/zoom, cast integration, depth/outline orchestration, resource cleanup |
| `client/src/lib/render/terrain.ts` | One-cell UVs, variants, chunked tops and sides, tier support |
| `client/src/lib/render/standees.ts` | Replace old basis/anchors with normalized object render layers and shared cutout materials |
| `client/src/lib/render/decals.ts`, `seen.ts` | Ground-overlay elevation, correct dirty keys, effective visibility |
| `client/src/lib/render/prerendered.ts` | Retire from normal runtime after conversion coverage; do not repurpose as a second scene mode |
| `client/src/lib/assets/catalog.ts` | Typed v2 catalog, revision-aware refresh, diagnostic errors, texture ownership |
| `client/src/lib/editor/BuildMode.svelte`, `draft.svelte.ts`, `brush.ts`, `scatter.ts` | Three-category library, preview/placement, heights, stairs, draft revisions, undo/publish semantics |
| `client/src/lib/campaign/CampaignPanel.svelte`, `ui/TokenPanel.svelte`, character UI | Cast selection, intentional placement, appearance inheritance, facing, elevation information |
| `client/src/lib/ui/ViewControls.svelte`, `table.svelte.ts`, `App.svelte` | Remove scene rotation/view cycling; preserve pan/zoom, scene state, and app workflows |
| `server/src/assets.rs`, `app.rs` | Versioned validation, safe install/catalog refresh, immutable IDs, isolate invalid packs |
| `server/src/map/layout.rs`, `scene.rs`, `compile.rs`, `drafts.rs` | Canonical layout, levels, objects/connections, compiler, revisions, support/navigation geometry |
| `server/src/map/package.rs`, `presentation.rs`, `uvtt.rs` | Isolated legacy readers/converters as needed; no active DA presentation pipeline |
| `server/src/net/protocol.rs`, `ws.rs` | Import/library endpoints, new validated messages, authoritative cast render records, revisions |
| `server/src/session/state.rs`, `permissions.rs` | Appearance resolution, movement, scene publication, permissions, cache invalidation, hidden-cast safety |
| `server/src/vision/*` | Finite-height LOS, visible masks, explored memory, shared authorization semantics |
| `server/src/systems/def.rs` and relevant creation/save paths | Default cast appearance separate from rule statistics; retain character compatibility |
| `server/src/storage.rs` | Explicit version migration, non-overwriting backups, atomic safe persistence, failure recovery |
| `client/src/lib/net/bindings/*` / generated Rust bindings | Regenerate from Rust declarations; no hand editing |
| `assets/packs/*`, `docs/assets.md`, `README.md`, `CHANGELOG.md` | Diagnostic assets, implemented contract, one-camera documentation, user workflow, change record |

Suggested focused modules: `cameraProfile.ts`, `objectLayer.ts`, `castLayer.ts`, `spriteMaterial.ts`, `occlusionOutlines.ts`, `surfacePicking.ts`, `assetValidation.rs`, `asset_import.rs`, `navigation.rs`, `height_visibility.rs`, and `migrations.rs`. Avoid expanding the existing large `engine.ts` and `state.rs` with every new concern.

---

## 14. Illustrative target pack manifest

This example describes the **proposed v2 contract**, not something the inspected application can already import. The coding agent must implement and validate the final contract and update this example to match it. File names are illustrative; this document does not supply those image files.

```json
{
  "schemaVersion": 2,
  "id": "darkwood-demo",
  "name": "Darkwood Demo",
  "visualProfile": "dimetric-30-v1",
  "filtering": "linear",
  "tiles": [
    {
      "id": "forest-floor",
      "name": "Forest Floor",
      "kind": "ground",
      "sourceSizePx": [128, 128],
      "coverageCells": [1, 1],
      "files": ["tiles/forest-a.png", "tiles/forest-b.png"],
      "transitionPolicy": "organic"
    }
  ],
  "objects": [
    {
      "id": "oak",
      "name": "Ancient Oak",
      "kind": "static",
      "placement": "cell",
      "footprintCells": [2, 2],
      "originCells": [1, 1],
      "pixelsPerWorldUnit": 128,
      "renderLayers": [
        {
          "kind": "upright",
          "file": "objects/oak.png",
          "sizePx": [256, 384],
          "pivotPx": [128, 368],
          "rootOffsetWorld": [0, 0, 0]
        }
      ],
      "blocksMovement": true,
      "blocksVision": true,
      "occludesVisually": true,
      "collisionPolygon": [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]],
      "visionVolumes": [
        {
          "polygon": [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]],
          "baseHeight": 0,
          "height": 3
        }
      ]
    }
  ],
  "cast": [
    {
      "id": "jagged-knife-bandit",
      "name": "Jagged Knife Bandit",
      "pixelsPerWorldUnit": 128,
      "suggestedFootprintCells": [1, 1],
      "views": {
        "north": {"file": "cast/bandit/north.png", "sizePx": [128, 256], "pivotPx": [64, 240]},
        "east":  {"file": "cast/bandit/east.png",  "sizePx": [128, 256], "pivotPx": [64, 240]},
        "south": {"file": "cast/bandit/south.png", "sizePx": [128, 256], "pivotPx": [64, 240]},
        "west":  {"file": "cast/bandit/west.png",  "sizePx": [128, 256], "pivotPx": [64, 240]}
      }
    }
  ]
}
```

Local footprint/vision polygons above are in ground world units relative to the object's origin. They are not image pixels. For the 2 × 2 oak, the center origin is `[1,1]` relative to its placement footprint's top-left; the illustrative trunk is much smaller than the visual footprint. Sprite ink size, visual height, and root placement must still be checked in the calibrated preview; valid JSON alone does not guarantee convincing art.

The final schema must also include wall/door orientation/state definitions, connection/stair records, ground overlays, and optional metadata such as attribution. Do not treat the abbreviated example as the entire implementation scope.

---

## 15. Definition of complete

The work is complete when the following demonstration succeeds on a fresh install and an upgraded save:

> Import a pack. Create a fixed 30° dimetric scene. Paint 128px source tiles. Place connected walls, a door, a tree, a table, and a barrel. Create a raised terrace and a higher ruin connected by stairs. Assign four-view cast assets to a PC and an adversary and place them on valid surfaces. Publish. Join as a player. Move using legal routes, see height-aware fog, see the silhouette of a permitted cast obscured by scenery, and see no trace of a deliberately hidden adversary. Open the door. Change scenes. Restart. Everything remains consistent.

A camera screenshot alone does not satisfy this definition. A sprite importer alone does not satisfy it. Geometry without server-side movement and visibility does not satisfy it. A generic outline around a transparent quad does not satisfy it.

### Required handoff artifacts from the coding agent

- Implementation commits and a milestone completion record.
- Current asset schema, template, validator, and actual diagnostic pack.
- The acceptance scene and reproducible browser screenshots.
- Migration report/example and recovery instructions.
- Test results and measured performance notes, including any pre-existing failures.
- Updated asset-authoring and GM scene-building documentation.

---

## 16. Evidence and confidence notes

The source audit used the exact commit recorded at the top. In particular, camera and selection behavior were inspected in `render/engine.ts` and `render/projection.ts`; texture behavior in `render/terrain.ts`; cutout placement in `render/standees.ts`; manifest rules in `server/src/assets.rs`; layout/blocking in `map/layout.rs` and `map/compile.rs`; visibility and scene state in `session/state.rs` and `session/permissions.rs`; publication routes in `net/ws.rs`; persistence in `storage.rs`; and cast/editor entry points in `CampaignPanel.svelte` and `draft.svelte.ts`.

Primary library references checked for feasibility: Three.js documentation for `OrthographicCamera`, `Raycaster`, `DepthTexture`, `Material`, and `OutlinePass`. Use APIs compatible with the repository's locked dependency version. An available outline API does not establish that an RGBA-sprite use case works without testing.

The architecture, limits, milestone estimates of complexity, proposed schema, and acceptance thresholds in this plan are design recommendations. They are not reported runtime measurements. The largest validation risks are cutout depth consistency, alpha-aware outlines, and height-aware visibility; the diagnostic scenes and milestone gates intentionally test those before production asset investment.
