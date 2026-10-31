# Rendering performance

The table draws on demand. Walking positions update with animation frames until the route finishes; stationary torch and magic effects update at most 15 times per second in the default power-saving configuration. Drawn character frames advance at their manifest rate: 4 fps for idle and 8 fps for the other starter actions. Idle runs for 2 seconds every 8 seconds, holding the first drawn frame between bursts with no animation wake-ups. A scene without animated lights or active poses sleeps. Hidden tabs cancel rendering and timers, then refresh when shown again; reduced-motion preferences hold a still pose.

The default drawing buffer uses at most one device pixel per CSS pixel and caps its longest edge at 1920 pixels. Terrain and repeated prop artwork already share geometry and textures. Changes to one character do not rebuild the map.

The forest revision adds three caches:

- Up to eight decoded elevation grids, keyed by their immutable encoded contents. Moving feet, hit tests and lights reuse them; painting a height gets a new grid immediately.
- Light colors, height, source selection and glow markers stay prepared while the map and camera are unchanged. Only pulse values change each tick.
- Character silhouette masks are reused during light flicker. Movement, facing/artwork, frame UVs, deformed fallback vertices, camera, visibility and scenery changes invalidate the mask. Mask materials belong to their renderer and are released when characters leave or the renderer closes.

Drawn characters use a four-vertex, two-triangle card. Frame updates reuse its UV buffer instead of allocating images, geometry or materials per frame. Picking and silhouettes read the same atlas frame. A refcounted cache holds at most 12 atlas loads/textures, including pending requests; unused settled sheets are evicted first. Generic procedural fallback assets retain a small subdivided card and effects. Fog updates reuse their canvas, texture and map geometry; decoded visibility masks are cached. Server sight masks are shared between exploration updates and snapshots, while exploration itself remains private to each player seat.

## Movement routing

Walking uses bounded A* over the map grid, including object footprints, closed doors and stair links. The client caches collision grids and retains at most four footprint sizes per grid. Repeating the same pointer destination returns the previous route. The largest built map has 40,000 cells; its cached numeric navigation arrays use about 1.7 MB with all four footprint sizes populated, plus temporary search and route data. A door or map change replaces the affected cache.

A CPU-only Node/Vite check of the saved 26 × 22 forest layout, with 128 collision segments, took 2.381 ms for the first route and a median 0.055 ms (maximum 0.124 ms) for subsequent changed destinations. A deliberately difficult 200 × 200 map with ten offset barriers needed a 920-point detour and measured 25–37 ms median for warm searches under concurrent work. These are measurements on the cloud CPU, not browser frame rates or minimum-hardware guarantees.

## Reproduce the checks

Run the server with the forest pack and the development client, then open `/perf.html?lighting=1`. The fixture loads real forest artwork, 18 trees, eight characters and two animated sources at a 1280 × 720 drawing buffer. It reports frame timings, the graphics driver, texture/geometry counts, scene passes and silhouette-mask passes. `/occlusion.html` and `/lighting.html` verify the corresponding rendered pixels.

`/animations.html` shows all eight characters and six poses, with a selector for each of the four directions. It checks frame-aware alpha picking, transparent intersections, WebGL errors and retained resource counts across the 192 character/view/action combinations. These checks establish correctness and retention, not a target hardware frame rate.

The final drawn-sheet check found all 768 frames alpha-pickable, rejected 619 transparent intersections, and reported no WebGL errors. After warming all directions, its 48 displayed cards retained 48 uploaded geometries and 12 textures, with eight atlases actively shared by the six action rows. Repeated playback kept those counts stable.

From `client`, run `node scripts/fog-render.mjs` for browser pixel checks of the production fog rendering. The fixture verifies different west/east viewpoints, a closed/open door, a boulder's visible near face, opaque unknown ground, dim remembered ground and props, and retained scenery on sight changes. The matching server tests calculate real LOS; the browser fixture uses synthetic masks to isolate rendering.

Before character animation was added, the cloud software-rendering check rebuilt the character mask zero times over 16 stationary light frames (previously 16 times). The unlit idle check recorded zero extra frames and retained 29 textures and 26 geometries. Those are historical baseline measurements; active character poses now intentionally invalidate the cast mask, and their retained effect meshes add geometry. These counts describe the fixture, not every campaign.

The cloud machine uses SwiftShader rather than a hardware GPU. Its frame timings varied substantially under concurrent browser work, so they do not establish a frame-rate guarantee for low-end devices. Actual hardware testing is still needed for minimum-system requirements. The fixture reports its renderer and drawing-buffer size so results can be compared on the same device and scene.

Built maps share their obstacle layout with clients, which reconstruct the walk animation along the route. The server also enforces routes on older imported image maps. Those maps omit wall geometry from player snapshots, so player-side animation cannot reproduce every server detour; the authoritative destination and collision checks still apply.
