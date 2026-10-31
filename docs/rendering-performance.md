# Rendering performance

The table draws on demand. Static miniatures do not schedule idle frames. They use a four-vertex, two-triangle card with shared directional textures. Smooth route interpolation runs only while a figure moves. Torch and magic pulses normally update at most 15 times per second in power-saving mode, and pause their background ticks while dice roll. Camera, state and movement updates still render. Hidden tabs suspend rendering; reduced-motion preferences skip route interpolation.

The default drawing buffer uses at most one device pixel per CSS pixel and caps its longest edge at 1920 pixels. Terrain and repeated prop artwork share geometry and textures. Updating one character does not rebuild scenery. Fog reuses its canvas and texture; decoded elevation and visibility grids are cached. Silhouette masks are reused across light flicker and invalidated by camera, visibility, facing, movement and scenery changes.

## Dice tray

The tray creates its WebGL renderer on the first roll. It uses a fixed 1× pixel ratio, no multisample antialiasing, one body mesh and one combined number mesh per die. Number atlases use 128-pixel cells rather than separate 256-pixel textures for every face. A tray retains at most eight warm atlases and six standard shapes; excess textures are released after their roll. Badges and result halos are released between rolls, and every retained resource is released on disposal.

Server-authoritative keyframes and face values are unchanged. Playback interpolates them on a monotonic local clock anchored to the server at roll start, preventing later clock corrections from causing jumps. Motion plays at 1.35× with a shorter lineup glide. Bounce sounds follow the same speed. Once result badges settle, no further frames are scheduled. Hidden tabs stop rendering and catch up when shown again.

A browser comparison of ten identical two-d12 rolls measured **36 → 14 draw calls**, **26 → 4 uploaded textures**, and **32 → 9 uploaded geometries** after settling. The optimized counts remained unchanged from the first to the tenth roll, with zero idle frames and no WebGL errors. Both trays used the same server meshes and orientations at 540 × 300 on the cloud SwiftShader renderer. These are fixture counts, not total application memory or a hardware FPS claim.

The static forest check recorded zero extra frames over an 800 ms stationary window with lights disabled (37 textures, 35 geometries). Re-enabling sources and then marking dice as rolling suspended background light ticks. All four cast directions rendered without WebGL errors.

## Movement routing

Walking uses bounded A* over the map grid, including object footprints, closed doors and stair links. The client caches collision grids and retains at most four footprint sizes per grid. Repeating the same pointer destination returns the previous route. The largest built map has 40,000 cells; its cached numeric navigation arrays use about 1.7 MB with all four footprint sizes populated, plus temporary search and route data. A door or map change replaces the affected cache.

A CPU-only Node/Vite check of the saved 26 × 22 forest layout, with 128 collision segments, took 2.381 ms for the first route and a median 0.055 ms (maximum 0.124 ms) for subsequent changed destinations. A deliberately difficult 200 × 200 map with ten offset barriers needed a 920-point detour and measured 25–37 ms median for warm searches under concurrent work. These are measurements on the cloud CPU, not browser frame rates or minimum-hardware guarantees.

## Reproduce the checks

Run the server and client, then open `/perf.html?lighting=1`. This loads real forest artwork, 18 trees, eight characters and two light sources at 1280 × 720 and reports timings and retained graphics resources. `/occlusion.html` and `/lighting.html` exercise their pixel effects. There is no animation showroom.

From `client`, run `node scripts/fog-render.mjs` for production fog pixel checks. Server tests verify real LOS; the browser fixture isolates rendering with synthetic masks. Client tests verify batched dice geometry and label UVs, including d4 corner numbers.

Cloud browser checks use SwiftShader software rendering. Measurements establish draw-call reductions, correct behavior and bounded retention; minimum hardware frame-rate claims require testing on actual devices.

Built maps share obstacle layouts so clients reconstruct sliding motion along the route. Imported image maps may omit walls from player snapshots; authoritative server collision checks still apply.
