# Rendering performance

The table draws on demand. A stationary scene with no animated lights stops drawing once artwork has loaded. Walking uses animation frames until the route finishes; stationary torch and magic effects update at most 15 times per second in the default power-saving configuration. Hidden tabs cancel rendering and light timers, then refresh when shown again.

The default drawing buffer uses at most one device pixel per CSS pixel and caps its longest edge at 1920 pixels. Terrain and repeated prop artwork already share geometry and textures. Changes to one character do not rebuild the map.

The forest revision adds three caches:

- Up to eight decoded elevation grids, keyed by their immutable encoded contents. Moving feet, hit tests and lights reuse them; painting a height gets a new grid immediately.
- Light colors, height, source selection and glow markers stay prepared while the map and camera are unchanged. Only pulse values change each tick.
- Character silhouette masks are reused during light flicker. Movement, facing/artwork, camera, visibility and scenery changes invalidate the mask. Mask materials belong to their renderer and are released when characters leave or the renderer closes.

## Movement routing

Walking uses bounded A* over the map grid, including object footprints, closed doors and stair links. The client caches collision grids and retains at most four footprint sizes per grid. Repeating the same pointer destination returns the previous route. The largest built map has 40,000 cells; its cached numeric navigation arrays use about 1.7 MB with all four footprint sizes populated, plus temporary search and route data. A door or map change replaces the affected cache.

A CPU-only Node/Vite check of the saved 26 × 22 forest layout, with 128 collision segments, took 2.381 ms for the first route and a median 0.055 ms (maximum 0.124 ms) for subsequent changed destinations. A deliberately difficult 200 × 200 map with ten offset barriers needed a 920-point detour and measured 25–37 ms median for warm searches under concurrent work. These are measurements on the cloud CPU, not browser frame rates or minimum-hardware guarantees.

## Reproduce the checks

Run the server with the forest pack and the development client, then open `/perf.html?lighting=1`. The fixture loads real forest artwork, 18 trees, eight characters and two animated sources at a 1280 × 720 drawing buffer. It reports frame timings, the graphics driver, texture/geometry counts, scene passes and silhouette-mask passes. `/occlusion.html` and `/lighting.html` verify the corresponding rendered pixels.

On the cloud software-rendering check, the same 16 stationary light frames previously rebuilt the character mask 16 times; the revised renderer rebuilt it zero times. The unlit idle check recorded zero extra frames. The run retained 29 textures and 26 geometries. These counts describe the fixture, not every campaign.

The cloud machine uses SwiftShader rather than a hardware GPU. Its frame timings varied substantially under concurrent browser work, so they do not establish a frame-rate guarantee for low-end devices. Actual hardware testing is still needed for minimum-system requirements. The fixture reports its renderer and drawing-buffer size so results can be compared on the same device and scene.

Built maps share their obstacle layout with clients, which reconstruct the walk animation along the route. The server also enforces routes on older imported image maps. Those maps omit wall geometry from player snapshots, so player-side animation cannot reproduce every server detour; the authoritative destination and collision checks still apply.
