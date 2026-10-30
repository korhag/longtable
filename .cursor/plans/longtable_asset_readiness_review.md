# Longtable — asset readiness and lightweight-rendering review

**Review date:** 29 October 2026  
**Repository:** `korhag/longtable`  
**Branch / release inspected:** `main` / `v1.2.0`  
**Pinned commit:** `a26bbd0b9c1d1e14135bf61f31bc0d4566d8fb51`  
**Compared with:** the previous fixed-30° implementation plan, based on `59b07fb6d3af939c48362e2edeeda0b3543c053f`.

## Verdict

**Import-and-preview ready in the inspected source; not yet ready for the requested import → build a scene → assign PC/adversary art → play workflow. Low-end performance is not yet demonstrated.**

The work is useful and should be retained. The fixed camera, schema-2 asset definitions, ZIP importer, basic validation, immutable conflicting-pack handling, and asset-library previews exist. However, the new asset categories have not been connected to the existing scene editor, gameplay compiler, and cast renderer. Merely adding attractive production images will not close that gap.

The next deliverable should be a **working flat sample-scene acceptance gate**, followed by measured performance hardening. Elevation can remain a later milestone without changing the chosen camera or abandoning the original plan.

No production artwork or sample art pack was created during this review. No repository files were changed.

### Evidence and execution limits

This is primarily a source audit using GitHub reads pinned to the commit above. The branch was checked again and still resolved to that commit. The previous plan was read from the provided Markdown attachment.

A local clone could not be obtained because this environment could not resolve GitHub, and there is no Rust toolchain available. Consequently **the application build, Rust tests, client tests, actual two-client workflow, and GPU benchmarks were not executed here**. Test counts in `docs/milestones/G0-G1.md` are the repository author's recorded results, not newly reproduced results.

An independent arithmetic check of the current camera depth range was executed; it is described below and supplied as `camera_depth_arithmetic.json`. A separate attempted GLSL browser probe did not initialize the graphics environment and produced no usable result. The shader concern below remains source-based and must be verified with the project's installed dependency version.

Source references such as **[S4]** resolve to exact paths under the pinned commit in the source index at the end. Function names are provided to avoid ambiguous line numbers.

---

## 1. Progress against the original plan

| Original goal | Current assessment | Evidence / qualification |
|---|---|---|
| G0: baseline and diagnostic harness | Present in source and documented | Fixture/smoke infrastructure and recorded tests exist. The smoke runner uses Chromium/SwiftShader; that is not a real-device performance benchmark. [S1, S19] |
| G1: fixed camera and scaffolding | Largely implemented, not complete scene readiness | `cameraProfile.ts` fixes 30° elevation / 45° azimuth. The main engine uses it. Runtime picking remains flat; large-map clipping and asset anchors still need attention. [S2, S3] |
| G2: contract, import, browser | Partially implemented | Schema-2 types, a GM import route, basic validation, previews and revisions exist. Validation, player-side refresh, and the full asset inspector are incomplete. [S4–S8] |
| G3: one-cell tiles and tile editor | Not completed | Renderer still resolves `pack.terrains`, reads the first file and repeats it over four cells. Editor tools still select legacy terrains. [S9, S10] |
| G4: objects and independent blockers | Not completed | Legacy props/walls/doors remain the live path. New `objects` metadata is not the source used by placement or the server compiler. [S9, S11, S12] |
| G5: cast artwork and assignment | Not completed | Token wire data has no cast-asset or facing field. Renderer still displays portrait discs. [S3, S13] |
| G6: obscured silhouette outlines | Not completed | Ground rings/halos are not sprite-shaped, depth-tested hidden outlines. [S3] |
| G7–G8: tiers, stairs and height-aware sight | Scaffolding only | Optional `levels` exists but is neither validated in layout validation nor used by the inspected terrain/picking/compiler paths. Sight remains the 2D polygon path. [S3, S12, S14, S16] |
| G9: unified publish/migration/cleanup | Not completed | Publishing still allocates a fresh scene ID; old presentation/import structures remain. Version-1 storage remains in place rather than the planned canonical migration. [S3, S7, S16, S17] |
| G10: acceptance scene, benchmark, authoring handoff | Not demonstrated | No integrated asset-ready scene or measured low-end benchmark was established by this audit. The guide still contains the retired contract after its opening notice. [S1, S18, S19] |

**Do not interpret this table as a percentage-complete estimate.** The missing integration goals carry much more user-visible weight than adding type definitions.

---

## 2. What would happen with a new sample pack today?

A pack like `docs/schemas/pack-v2.example.json`, supplied with the required PNGs, follows the new importer path. The library can display the ground image, object layers and four cast views. This is a useful importer test. [S4–S8, S20]

However:

1. Selecting the new floor in `AssetLibrary.svelte` only updates its local `selected` value and opens information. It does not select a painting brush.
2. `BuildMode.svelte` builds its paint/place lists from `pack.terrains`, `pack.props`, `pack.walls`, `pack.doors` and `pack.decals`, not the new `tiles` and `objects` arrays.
3. `terrain.ts` resolves the old terrain list, and `standees.ts` resolves the old prop/wall/door lists.
4. The server compiler calls `Catalog::prop`, which only searches legacy props. New object collision and sight metadata therefore does not become gameplay geometry.
5. There is no cast-asset reference in `TokenView` and no runtime direction selector. In `TableEngine.sync`, `standing` is explicitly set to `false`; `paintToken` displays the disc and hides the upright card.

**A successful import or attractive thumbnail is therefore not proof that the map can use the asset.** [S3–S13]

Do not work around this by requiring every new pack to duplicate its content into both old and new categories. A temporary, explicitly tested adapter can be acceptable internally, but artists should receive one stable contract, not parallel manifests with conflicting dimensions and behaviors.

---

## 3. Findings that block the sample-scene workflow

### F1 — The new catalog is disconnected from scene authoring and gameplay

**Priority:** blocks the requested sample scene.  
**Confidence:** confirmed in source.  
**Locations:** `AssetLibrary.svelte`; `BuildMode.svelte`; `terrain.ts`; `standees.ts`; `assets.rs::Catalog::prop`; `map/compile.rs::compile`.

Connect imported asset selection to the editor's tool/selection state and ghost placement. Resolve canonical v2 references through shared, typed lookup helpers. Use the same resolved definitions for rendering and server-side placement/collision validation. Reject missing or wrong-category references instead of silently skipping their gameplay blockers.

**Acceptance:** using only the v2 example categories, import a pack, paint its ground, place its barrel, publish, and observe both art and movement behavior. Repeat on a player connection and after restart. No hand-editing a scene or legacy duplicate definitions should be necessary. [S4–S12]

### F2 — Cast frames exist as catalog records, not usable scene appearances

**Priority:** blocks PCs/adversaries with body artwork.  
**Confidence:** confirmed in source.

Add authoritative character appearance/default and token override references, facing, and the appropriate rendering dimensions/pivots. Preserve portrait fields separately. Connect artwork assignment in the character/adversary workflow to intentional scene placement. Implement four-direction idle sprites without requiring animation or equipment attachment.

The existing `view.cast` / `CastPaint` naming relates to **ring coloring**, not to the new catalog's four-view cast artwork. Do not mistake it for a completed cast-asset renderer.

Use one shared texture cache across repeated bandits; creating several tokens with the same art must not decode/upload an independent copy per token. Regenerate TypeScript bindings from Rust. [S3, S6, S13, S15]

**Acceptance:** two PCs and several adversaries can choose distinct cast assets, move and face consistently for all viewers, share artwork without sharing state, retain appearance after reconnect/restart, and remain correctly grounded while zooming.

### F3 — The old terrain scale and rendering path remain

**Priority:** blocks accurate one-cell asset authoring.  
**Confidence:** confirmed in source.

`terrain.ts` still contains `fract(warped / 4.0)` and `found.def.files?.[0]`. The texture is sampled over four ground cells, not the one-cell coverage declared by the new pack. All ground also passes through a warping path; rigid cobbles should not be deformed by an organic terrain transition technique. [S10]

Implement explicit one-cell coverage and deterministic variants. Keep rigid UVs fixed. Transition blending should affect boundary masks, not bend paving stones. Respect filtering and use consistent color handling.

**Additional shader concern:** this file sets `glslVersion: THREE.GLSL3` but writes `gl_FragColor` without declaring a fragment output. The inspected upstream Three.js shader-prefix implementation omits the compatibility alias when GLSL3 is explicitly selected. Treat this as a high-priority shader-compile check against the lockfile's actual Three.js version; declare/use an explicit output in GLSL3. This was not executed inside the full application here. [S10, E4]

**Acceptance:** the real `TerrainLayer`, not only an unrelated diagnostic quad, renders a known-color textured scene without shader errors. A marked 128×128 source repeats exactly once per cell. Variants remain stable across clients, reloads and unrelated edits.

### F4 — Object occlusion, body silhouettes and correct anchors need completion

**Priority:** required for the visual sample-scene gate.  
**Confidence:** missing functionality confirmed in source; final appearance requires browser testing.

The existing cutout scale has been adjusted for the chosen view, which is useful. But the standee placement anchor still uses the old front-left convention, and runtime object construction still uses legacy fields. New root offsets, render layers, independent collision polygons and finite-height sight volumes are not wired through the live compiler. [S6, S11, S12]

Implement the original plan's upright, horizontally view-aligned cutouts and explicit pivots. Use alpha-aware depth, so transparent tree gaps do not behave like opaque rectangles. Keep trunk collision independent of canopy width.

Implement permitted-cast silhouette outlines. Ground rings and selection halos do not satisfy this requirement. The outline must follow the body alpha and only appear where the body is camera-obscured. Hidden or server-withheld adversaries must never enter outline, ID, label or hover buffers.

**Acceptance:** walk permitted cast in front of and behind a tree and barrel; check alpha holes, partial/full occlusion, selection via the outline, and hidden-enemy non-disclosure. Check a door in both states and both wall axes.

### F5 — Catalog refresh is tied to the GM's asset-library component

**Priority:** required before a multiplayer asset-ready claim.  
**Confidence:** confirmed source-path gap.

The server broadcasts a catalog revision and the table store records it. `AssetLibrary.svelte` calls `noteCatalogRevision()` in an effect, but that component lives in the GM map editor. `App.svelte` loads catalog data initially only when `editor.packs` is empty. An already-connected player with a nonempty catalog does not have the same refresh path. [S4, S5, S7, S15]

Move catalog revision handling into a shared store active for every connected client. Include table/server/session changes in cache lifecycle. Make loading retryable, avoid refresh races, and never preserve a failed request as a permanent empty catalog. Prefer a string content revision/hash or a safely bounded integer rather than exposing an unrestricted Rust `u64` as a JavaScript number.

**Acceptance:** connect GM and player first; import a new pack afterward; place its art without page reload. Both clients acquire the same definitions and images. Also test reconnect, import failure, retry and a scene switch during loading.

### F6 — The importer is useful but not yet a complete asset validation boundary

**Priority:** high for reliability and low-memory hardware.  
**Confidence:** confirmed in source.

Keep the existing good checks: profile/schema, referenced frames, ground dimensions, duplicate IDs, archive path restrictions, compressed/expanded archive limits and immutable conflicting imports. Complete the following:

- `MAX_SIDE = 2048` is enforced in the tile-validation loop, not for object/cast images. Apply side, pixel-count and decoded-byte limits to **all** image classes.
- `decoded()` calls full image decoding before dimensional budgeting. Inspect dimensions and enforce decoder allocation limits before full decode. Check aggregate decoded memory, not just compressed or archive-expanded file bytes.
- Validate declared portraits by decoding them, not merely checking file existence. Verify supported alpha/color formats; make opaque cast/object art an actionable error or an explicitly acknowledged warning.
- Validate positive/bounded `pixelsPerWorldUnit`, footprint dimensions, overlay/side coverage, finite offsets, valid polygons/volumes, supported layer kinds and behavior metadata. Fields that are accepted but ignored must not imply working gameplay support.
- Define whether pivots use pixel-center or image-edge coordinates and make the importer, preview and authoring guide agree. The current validator rejects `pivot == image size`, while the retired guide uses bottom-edge anchors.
- Keep import file IO/decoding/hashing out of async request-worker hot paths. Use bounded blocking work. A new import currently reloads and decodes existing v2 packs again via `load_catalog`.
- Apply the compressed limit while streaming the upload, not only after collecting the complete multipart field; the shared route currently allows a larger body than the pack-specific cap.
- Serialize conflicting same-ID installation or use unique staging directories and guaranteed cleanup. Exclude staging directories from catalog discovery.

These are robustness improvements, not a claim of an observed exploit or an observed out-of-memory crash. [S6–S8]

### F7 — Publish/update semantics and authoring documentation are not finalized

**Priority:** required for a trustworthy scene workflow.  
**Confidence:** confirmed in source.

`publish_built()` still creates a new scene ID, while `install_scene()` retains and clamps existing tokens. Distinguish new-scene creation from updating a specific existing scene; preserve valid placements/state and reject/report invalid ones. Keep campaign/scene switching and persistence intact. [S16, S17]

`docs/assets.md` warns that its old contract is retired, but its main body still supplies the old 53.13° formulas, four-cell terrain and legacy anchor examples. Replace those sections with one production guide rather than instructing future asset generation from a contradictory page. Provide real repeat/pivot/depth previews. [S18]

**Acceptance:** build, publish, place cast, reopen/edit and republish the same scene without ID churn or unexplained relocation; restart and repeat. The guide must generate a pack that is both accepted and visibly usable by the same runtime.

---

## 4. Efficiency findings and targeted changes

The chosen fixed-camera cutout approach does not require a full animated 3D world. Keep Three.js and the current lightweight direction. The main risks found are repeated CPU work, GPU-resource lifetime, redraw scheduling and incomplete integration—not an inherent need for a powerful discrete GPU.

### P1 — Async asset completion does not consistently schedule a frame

**Observed:** `TerrainLayer.paintFiles`, `StandeeLayer.swap`, decal texture callbacks and token portrait callbacks change textures/materials/meshes but do not request an engine redraw. `TableEngine` only renders when its `dirty` flag is true. [S3, S10, S11, S21]

**Consequence:** a load that finishes after the last dirty frame can leave the displayed scene stale until another interaction or state update. This is a source-derived failure condition, not a captured full-app reproduction.

**Fix:** one coalescing `invalidate()`/frame scheduler shared by layers. Trigger it on successful/error asset completion, changes to visible state, viewport changes and active animation. Add cancellation/generation guards to prevent late texture responses replacing disposed/newer content. Do not fix this with unconditional rendering forever. [E1]

**Test:** delay image responses, then leave the pointer still. Final art must appear without a click, pan or network state change.

### P2 — Whole-terrain reconstruction for a paint edit

**Observed:** the terrain cache key includes the entire cells string. A change calls `clear()`, reallocates the texture array and cell texture, rebuilds geometry/materials, then runs `paintFiles` again. [S10]

**Fix:** cache decoded terrain source images and texture layers independently of layout edits. Update the changed cell-data region or bounded terrain chunks. Coalesce brush input to one update per frame; avoid fresh image/canvas processing for every brush movement.

**Test:** paint 100 strokes with the same materials. Source textures must not be decoded repeatedly and GPU texture count must not grow with stroke count.

### P3 — Scenery rebuilding on exploration or door changes

**Observed:** the standee cache key includes `fog.explored`, all props and pieces serialized to JSON, and all door states. A key change clears/recreates the entire group. Corner posts are discovered with a nested scan over wall pieces. [S11]

**Fix:** keep instances keyed by placement ID; update only changed transforms/door art/visibility. Separate fog state from static geometry. Index wall junctions by grid coordinate instead of all-pairs scans. Use explicit revisions rather than repeatedly serializing whole arrays.

**Test:** open one door or reveal one fog cell in a populated map. Unrelated object geometry and materials must retain identity.

### P4 — Resource lifetime and unbounded retention

**Observed:** removal of tokens only detaches their groups and deletes map entries. It does not dispose token geometry, materials and owned textures. `TableEngine.dispose()` does not explicitly clean all remaining object/decal/token/fog resources. Replaced board materials and wall-line materials have incomplete disposal paths. The global asset texture cache has no eviction/lifetime policy. [S3, S5, S11]

Detaching an object does not automatically free its Three.js GPU resources. Shared resources also must not be disposed while another instance needs them. [E2]

**Fix:** explicit ownership/reference counting or bounded scene/pack lifetime policies; comprehensive `dispose()` paths; generation-safe asynchronous loading; reuse common geometry/materials. Remove event listeners on teardown. Bound retained textures and account for render-target memory separately.

**Test:** 30 cycles switching between two populated scenes, with repeated cast creation/deletion. After warm-up and allowed cache retention, texture/geometry/material counts and estimated memory must plateau, not climb linearly.

### P5 — Unrelated state can rebuild scene components

**Observed:** the app passes full snapshot state to `engine.sync()`. Each sync calls walls, doors and lights updates; several of those clear/recreate their content. `syncTokens()` also walks all cast. A character-sheet/resource or other unrelated snapshot can therefore trigger unnecessary map work. [S3, S15]

**Fix:** give scene, geometry, fog, assets, cast appearance, cast transforms and selection distinct revisions/dirty categories. A ring-color change should update a ring, not rebuild scenery. Reuse unchanged visibility calculations on the server.

**Test:** change Hope/Stress, send chat, select a cast, and roll dice while watching scene rebuild counters. Static terrain/object rebuild counters should not increase unless their relevant inputs changed.

### P6 — Idle draws are gated, but the JavaScript frame loop never sleeps

**Observed:** `start()` schedules its next `requestAnimationFrame` before checking `dirty`. Thus it is **not continuously drawing the GPU scene**, but it does wake the callback every frame. [S3]

**Fix:** schedule one frame when invalidated and stop scheduling when settled. Permit animation-driven frames only while an animation is active. Pause unnecessary work in hidden tabs. Preserve prompt camera/drag response. [E1]

**Test:** settle the scene with no input or asset requests, then observe a 10-second interval: zero map draws, zero scene rebuilds and zero continuously scheduled map frames. Legitimate networking/heartbeat activity is a separate concern.

### P7 — Quality controls and hardware limits are not an acceptance contract

**Observed:** antialiasing is enabled and DPR is capped at 1.5. These are reasonable starting settings, not a guarantee of low-end performance. [S3]

**Fix:** an explicit low-power profile should allow DPR 1.0, bounded drawing-buffer resolution and simpler optional effects, without changing the camera or hiding essential cast outlines. Recreating the WebGL context may be necessary for a context-level antialiasing change; do not pretend it is a mutable material flag.

Avoid dynamic shadows, bloom, SSAO, per-frame CPU pixel readback, one full-screen outline target per actor, or a separate material per terrain cell. Cache static depth where correct; update bounded shared mask/depth targets only when needed. Batch or atlas repeated scenery **where profiling shows a draw-call problem**, preserving alpha/depth correctness.

Three.js's current `WebGLRenderer` needs WebGL2. The support promise should be **tested ordinary computers with a working hardware-accelerated WebGL2 browser**, not literally every computer. A software-rendered smoke pass does not establish smooth gameplay on old hardware. [S19, E3]

---

## 5. Other correctness checks found during the review

### A. Test-source compilation risk

In `server/src/asset_import.rs`, the test helper is declared as `temp_assets() -> PathBuf`, but the inspected file imports `Component` and `Path`, not `PathBuf`, and the test module has no separate `PathBuf` import. This appears to be an unresolved type in the test build. Add the test-module import and run the full Rust test suite. This is a source finding, **not a compiler run performed in this review**. [S8]

### B. Large-map camera near clipping

`placeCamera()` uses a fixed pull of 48 and near plane 0.05; `fitMap()` only changes orthographic extents. A large map can extend behind that camera plane. [S2, S3]

Independent calculation at centered focus and ground height zero:

| Map | Nearest ground-corner view depth | In front of near plane? |
|---|---:|---|
| 12×12 | 40.651531 | Yes |
| 50×50 | 17.381378 | Yes |
| 80×80 | -0.989795 | No |
| 200×200 | -74.474487 | No |

These numbers use `48 + dot(world - focus, forward)` with the source camera basis. They are not a rendered screenshot. Compute pull/near/far from supported world bounds, including tall assets/elevation and allowed pan, without changing the fixed orientation. Fit also currently passes rise `0`, so it does not account for tall scenery. The small initial acceptance scene does not need a larger-map artwork pack, but the advertised map-size limit needs this regression test.

### C. Decal invalidation omits meaningful changes

The decal cache key includes asset/position and show-all status, but omits scale, angle, fog changes and catalog revision. A previously hidden decal may not appear when its area becomes explored, and changed scale/angle can stay stale. Fix dirty categories without replacing this with full group reconstruction on every frame. [S21]

### D. Levels are not yet gameplay

The optional base64 `levels` field is not decoded/length-checked/range-checked by the inspected layout validator. Terrain is still a single flat surface and picking intersects `Y=0`. Do not announce three-level support on the strength of the field or the mathematical tier-plane unit tests. For the first flat-scene gate, reject unsupported nonzero levels clearly or keep them unavailable in authoring. Then complete G7/G8. [S3, S14]

---

## 6. Achievable implementation sequence for the coding agent

These milestones complete missing integration rather than replacing the original plan. Preserve unrelated account, chat, dice, character-sheet and rule-system changes. First compare the working checkout with the reviewed commit and re-check changed findings.

### R0 — Establish a reliable build and real-render baseline

Fix the `PathBuf` test-source issue if still present. Run Rust tests, Svelte/TypeScript checks, client tests and production build. Add a browser check using the **actual terrain/object/cast pipeline**. Verify GLSL3 output, capture shader errors, and fail a blank/no-art scene rather than merely printing a smoke success marker.

**Done when:** command logs and browser evidence identify the current baseline; no claimed pass comes from old milestone notes alone.

### R1 — Finish the asset boundary and all-client refresh

Complete importer budgets/validation, bounded background decoding and atomic install behavior. Use a shared revision-aware catalog for every client. Build reusable canonical lookup helpers and define unsupported behaviors explicitly.

**Done when:** valid packs appear on already-connected GM/player clients; invalid packs produce specific errors and leave installed packs unchanged; malformed/oversized imports cannot exceed configured allocation budgets.

### R2 — Make imported tiles and objects usable in a flat scene

Connect library selection to paint/place tools and ghosts. Implement one-cell tile mapping/variants. Render object layers with final pivots and independent blockers. Connect server compilation to the same canonical v2 records. Add depth-correct alpha-cutout traversal tests and both wall axes/door states. Fix source texture reuse and async redraw as part of this work, not afterward.

**Done when:** a v2-only pack creates a usable flat scene without manual JSON edits or legacy duplicate definitions.

### R3 — Add cast assignment, placement and directional artwork

Implement authoritative appearance references and facing, character defaults/token overrides, shared textures, correct root pivots and separate portraits. Add asset assignment to PCs/NPCs/adversaries and intentional placement.

**Done when:** several distinct PCs/adversaries display and move correctly on two clients and survive save/reload.

### R4 — Add safe silhouette outlines and alpha-aware selection

Implement the original plan's bounded outline/depth approach, with one consistent effective viewer. Avoid a full-screen target per cast. Server-withheld cast must be absent from all rendering and interaction caches. Preserve usability in the low-power profile.

**Done when:** partial/full foreground occlusion, alpha holes, overlapping cast, zoom/DPR and GM-as-player tests pass, including hidden-enemy non-disclosure.

### R5 — Prove stability and low-end efficiency

Use retained instance updates, chunked terrain where appropriate, cached images, reference-counted resource cleanup and a sleeping redraw scheduler. Fix decal invalidation and publish/update identity. Add memory/rebuild instrumentation and run the small and stress scenes below.

**Done when:** measurements meet an explicitly recorded low-end target, repeated switches plateau in resource use, and idle rendering sleeps.

### R6 — Freeze the authoring guide and declare the flat sample gate

Replace contradictory asset documentation. Document the exact supported manifest, pivots, world scale, object layers, tile repeats, facing labels and import workflow. Include a small deterministic diagnostic pack and scene in automated integration coverage. These can use test art; no polished art is needed to prove the pipeline.

**Done when:** someone following the guide can import/build/play the scene without touching source files. This is the point to create the requested attractive sample asset collection.

### After the flat gate — finish the original elevation and migration milestones

Complete G7/G8 for tiers 0–2, stairs, height-aware movement/sight/fog, and G9/G10 for the remaining migration/cleanup/full acceptance scene. Keep one walkable surface per cell and the same fixed camera. Do not claim the original plan fully complete merely because the flat gate passed.

---

## 7. Required acceptance and performance evidence

### Functional sample gate

Use a small v2 diagnostic pack sufficient for a 12×12 or 20×20 scene: several ground materials, one alpha ground overlay, a tree, a barrel/table, wall/door pieces, and at least two cast appearances. The purpose is integration testing, not delivering the future art pack in this review.

Test in GM, player and GM-as-player sessions:

- Import, paint, place and publish solely through the UI.
- Walk cast in front of/behind foreground art; verify roots, image scale, gaps and silhouette outline.
- Use cast art on linked PCs/adversaries, and reuse the same art across independent tokens.
- Test a hidden adversary and an out-of-sight adversary: no body, outline, label or click target.
- Join before an import, join after it, reconnect and restart.
- Edit/republish the same scene; switch between two scenes repeatedly without state loss.
- Delay image responses and change scenes mid-load; no stale image substitution and no required extra click to reveal loaded art.
- Validate a missing/corrupt image, bad pivot, zero scale, oversized image, unsupported profile, unknown asset reference and conflicting pack ID.

### Performance scenarios

| Scenario | Initial workload | Purpose |
|---|---|---|
| Small playable scene | 20×20 cells, about 100 objects/overlays, 8 cast | The upcoming modest art pack should feel responsive here |
| Representative stress | 50×50 cells, 500 objects, 50 cast | Preserve the original plan's scale test; measure object and outline costs |
| Supported-boundary check | 200×200 grid with bounded content | Camera clipping, allocations and editor validation; not a promise of unlimited art density |
| Lifecycle soak | 30 populated-scene switch cycles plus cast deletion/recreation | Find resource leaks and stale caches |
| Import-under-session | Two clients connected while a bounded pack is imported | Detect blocking server work and catalog refresh failures |

Test at 1920×1080 CSS viewport where practical, DPR 1 in low-power mode, and report actual drawing-buffer resolution. Use a representative integrated-GPU machine with normal hardware acceleration. Record CPU/GPU, RAM, OS, browser/version, power mode and scene contents. Test at least the deployment browsers you intend to support. SwiftShader is useful for correctness smoke tests, not GPU-performance sign-off.

**Initial proposed targets, not measurements:** aim for p95 active frame time around 16.7 ms for the small warmed scene (60 Hz target), and a documented low-power fallback that remains comfortably within about 33.3 ms (30 Hz). Investigate long input/main-thread stalls separately; average FPS can hide them. Do not silently lower the target without documenting the tested hardware and visual tradeoff.

Record: p50/p95/p99 frame intervals; main-thread long tasks; GPU timing where supported; draw calls and rendered primitives; resident texture estimates; geometry/texture counts; terrain/object rebuild counts; image decode/upload counts; server visibility/action times; and import CPU/peak memory. Browser resource counters are not exact total VRAM measurements—report estimates and counters honestly.

Hard behavioral gates regardless of chosen FPS target:

- No map draw/rebuild work while the settled scene is unchanged.
- Moving one actor does not rebuild terrain or every object.
- Opening one door does not recreate unrelated scenery.
- Repeated paint strokes do not reload every source image.
- Repeated scene switches do not cause linear retained-resource growth.
- Camera/panning/selection remain responsive while routine server events arrive.
- The full UI remains usable without a discrete GPU on the chosen supported reference machine.

---

## 8. Source index

All **S** paths below were read at commit `a26bbd0b9c1d1e14135bf61f31bc0d4566d8fb51`. Canonical URL prefix:

`https://github.com/korhag/longtable/blob/a26bbd0b9c1d1e14135bf61f31bc0d4566d8fb51/`

| ID | Path / locator |
|---|---|
| S1 | `docs/milestones/G0-G1.md` |
| S2 | `client/src/lib/render/cameraProfile.ts` |
| S3 | `client/src/lib/render/engine.ts` — `start`, `sync`, `dispose`, `syncBuilt`, `syncTokens`, `placeCamera`, `fitMap`, `hitMap`, `createToken`, `paintToken` |
| S4 | `client/src/lib/editor/AssetLibrary.svelte` |
| S5 | `client/src/lib/assets/catalog.ts` |
| S6 | `server/src/assets.rs` — v2 definitions, catalog lookup/loading |
| S7 | `server/src/net/ws.rs` — `import_pack`, catalog route, draft publishing |
| S8 | `server/src/asset_import.rs` — validators, archive install, image decode, tests |
| S9 | `client/src/lib/editor/BuildMode.svelte` |
| S10 | `client/src/lib/render/terrain.ts` |
| S11 | `client/src/lib/render/standees.ts` |
| S12 | `server/src/map/compile.rs` |
| S13 | `server/src/net/protocol.rs::TokenView`; `client/src/lib/net/bindings/TokenView.ts` |
| S14 | `server/src/map/layout.rs` |
| S15 | `client/src/App.svelte`; `client/src/lib/table.svelte.ts` |
| S16 | `server/src/session/state.rs` — `publish_built`, `install_scene`, `reveal_from_tokens` |
| S17 | `server/src/storage.rs` |
| S18 | `docs/assets.md` |
| S19 | `client/scripts/render-smoke.mjs`; `client/package.json` |
| S20 | `docs/schemas/pack-v2.example.json` |
| S21 | `client/src/lib/render/decals.ts` |

Primary external technical references consulted:

- E1 — Three.js, Rendering on Demand: `https://threejs.org/manual/pages/rendering-on-demand.html`
- E2 — Three.js, How to dispose of Objects: `https://threejs.org/manual/pages/how-to-dispose-of-objects.html`
- E3 — Three.js, WebGLRenderer: `https://threejs.org/docs/pages/WebGLRenderer.html`
- E4 — Three.js upstream shader prefix source, inspected `dev` blob `f423137d3c0e49cafcf1ad026a6a094ccd5f2c84`: `src/renderers/webgl/WebGLProgram.js`, conditional GLSL3 output/alias injection. This is supporting upstream source, not proof of the exact installed lockfile build; verify locally.

## Final decision

**Do not spend time finalizing a large production art collection yet.** Keep the fixed 30° dimetric direction. Finish R0–R6 with diagnostic art, prove the import-to-game path on two clients, and measure the low-power baseline. Then the proposed small, attractive collection of PCs, scenery and ground textures has a stable system to target.
