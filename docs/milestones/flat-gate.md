# Flat scene gate

This file records what was run for the flat asset scene. A green upload or a SwiftShader quad is not this gate.

Reviewed commit: `a26bbd0b9c1d1e14135bf61f31bc0d4566d8fb51` (v1.2.0). The working tree was checked again. The missing `PathBuf` import from that review was already fixed here, and the Rust tests compiled.

## Commands

| Command | Result |
| --- | --- |
| `cargo test --manifest-path server/Cargo.toml --offline` | 147 passed, 0 failed |
| `npm run check` in `client` | 0 errors |
| `npm test` in `client` | 26 files, 57 tests passed |
| `npm run build` in `client` | production build finished |
| `node client/scripts/scene-render.mjs` | `SCENE_OK` |
| `node client/scripts/occlusion-render.mjs` | `OCCLUSION_OK` |
| `node client/scripts/scene-perf.mjs` | `PERF_OK` on Intel UHD 630, numbers below |
| `node client/scripts/scene-perf.mjs --swiftshader` | earlier software-renderer check; not a frame-time result |
| `node client/scripts/ui-scene.mjs` | `UI_OK` |
| `npx vitest run src/lib/render/cameraProfile.test.ts` in `client` | 8 passed, including the 200×200 near/far check |
| `npx vitest run src/lib/assets/catalog.test.ts` in `client` | 1 passed. Asking for the same picture twice counts as one decode. |
| `node client/scripts/load-scene.mjs` | `LOAD_OK` |

`a_flat_scene_imports_compiles_publishes_and_reloads` is the authoritative scene test. It imports a schema-2 zip, compiles a barrel as a movement blocker and an oak trunk as sight, refuses stairs and a nonzero height grid, publishes twice with the same scene id, shows the cast picture to the player, omits the hidden token from the player snapshot, and reads the cast picture and the hidden token back from `state.json`.

`a_player_joins_the_published_scene_on_a_second_socket` opens two live sockets on `127.0.0.1`. The GM imports the pack while the player is connected, and both snapshots move from catalog revision `1` to `2`. The GM publishes the draft twice and the scene id stays the same. The player receives the barrel, Ada's bandit picture, and Bram's scout picture. The hidden figure never arrives. Moving Ada east updates the player's snapshot and leaves Bram facing south. When the GM looks through Ada's eyes, the hidden figure drops out of the GM snapshot and returns when that view is cleared. The save reloads the scene id, both pictures, the facing, and the hidden figure.

`scene-render.mjs` draws the real terrain shader and one upright cutout in headless Chromium. The center pixel of a red ground tile was red, so the GLSL3 fragment output compiled against Three.js 0.186.1. That run is SwiftShader at a 256×256 drawing buffer. It is a correctness check. The old smoke quad was not used as this result.

## Performance record

The measured machine is this computer: Linux 7.2.5-3-omarchy, 12 CPU threads, 31 GB RAM, Intel UHD Graphics 630 (Coffee Lake GT2). There is no discrete GPU on this machine. Browser: headless Chromium 152.0.7977.82, ANGLE on Mesa OpenGL ES 3.2. WebGL renderer string: `ANGLE (Intel, Mesa Intel(R) UHD Graphics 630 (CFL GT2), OpenGL ES 3.2)`. CSS host 1920×1080. Drawing buffer 1920×1080, so the device pixel ratio in this run is 1. Power profile: the default low-power profile.

Command: `node client/scripts/scene-perf.mjs`.

| Scene | What happened |
| --- | --- |
| 20×20, 100 objects, 8 cast | 1 terrain rebuild. The 100 barrels are one shared drawing. After pictures finished loading, a further 800 ms scheduled no extra map draw. Moving one actor did not rebuild terrain or objects. Opening one door rebuilt that door and did not rebuild the terrain. A pixel on a barrel was the barrel color. Active frames, after dropping the first five of forty: p50 16.6 ms, p95 17.8 ms, p99 18.5 ms. |
| 50×50, 500 objects, 50 cast | 1 terrain rebuild and 1 object rebuild for all 500 barrels. The check required those 500 drawn instances. Building the scene plus a 200 ms settle took 305 ms. Active frames, after dropping the first five of twenty-four: p50 16.6 ms, p95 17.8 ms, p99 17.8 ms. |
| 30 switches between a 12×12 and a 16×16 scene | Texture count 13 after the first scene and 13 after the thirtieth. |

The proposed small-scene budget was a p95 around 16.7 ms, and a low-power budget comfortably within 33.3 ms. On this chip both scenes have a median of 16.6 ms. Both p95 values are 17.8 ms, inside the 33.3 ms budget and 1.1 ms above a strict 16.7 ms cap. An earlier run of the same crowded scene, before matching objects shared one drawing, had a p95 of 74.4 ms. These are this run's numbers. They are not a claim that every computer is smooth.

An earlier `scene-perf.mjs --swiftshader` run drew the same scenes on SwiftShader at 1920×1080. That run checked rebuilds, idle sleep, and the texture plateau. It did not produce these frame times. SwiftShader is not this result.

`occlusion-render.mjs` draws the real outline pass at device pixel ratios 1 and 2. On SwiftShader, the outline appears where a green occluder is in front of a red figure and does not appear on the visible side. The same page loads the table engine: an opaque part of a figure can be selected, a transparent part cannot, a figure removed from the view cannot be selected, and both far corners of a 200×200 map stay in front of the camera when the view is centered on the opposite corner.

`ui-scene.mjs` drives the real buttons in two browser contexts on headless Chromium (SwiftShader). The GM imports a schema-2 zip, paints Floor, places a barrel, assigns the bandit picture, and puts the map on the table. A second person creates an account, accepts the friend request, asks for a seat, and sits down. That person's snapshot receives catalog revision `2`, the floor, the barrel, and the visible figure's cast picture. The hidden figure is absent. The GM looks through that person's eyes and comes back with Your eyes. Putting the map on the table a second time keeps the scene id, and `state.json` keeps the tile, the barrel, the cast picture, and the hidden figure. This is a button check, not the Intel frame-time measurement.

`a_player_does_not_receive_an_adversary_outside_sight` puts a wall between Ada and Shade, and leaves Far outside Ada's sight. Neither name is in the player's snapshot or in the GM's view through Ada's eyes. Near, on Ada's side of the wall, is present. The GM's own view still has Shade. A figure that never arrives has no body, outline, label, or click target.

`load-scene.mjs` holds pictures for a rose room, switches to a green room, then lets the rose pictures finish. The green room does not take the rose colors. When the green pictures finish, the barrel pixel is `(0,204,102)` and the figure pixel is `(51,153,255)`, and the redraw happens without a click. Thirty identical snapshots then take 7 ms and add no map draw, and the camera still projects a point. That run is SwiftShader. It checks picture timing, not frame time.

## Still open

Nothing in the flat scene gate is waiting on a measurement that missed its budget. The goals below are the original plan's elevation work.

## Remaining original-plan goals

These are not done, and they are not abandoned.

- Tiers 0–2 and stairs. The importer and the layout reject them with "not available yet".
- Height-aware sight and fog. A vision blocker is still an infinite wall, so it also stops walking.
- Full migration off the legacy terrain and prop lists. Old packs still load. New packs do not have to fill those lists.
- The elevated acceptance scene and production art.
