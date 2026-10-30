# Game Table

A self-hosted table for a roleplaying game. One Rust process on the GM's machine owns the map, the fog, and the dice. Browsers only draw what that process sends them, so a player cannot peek past a wall or invent a roll. The battle map is the first surface of the game. Characters and rules can be added on this same host.

Figures stand upright on the map. The camera stays at one angle. Character sheets and the dice tray are on this same host.

## Run it while building

Two processes, from this directory:

```bash
cargo run --manifest-path server/Cargo.toml
```

```bash
cd client && npm install && npm run dev
```

Open `http://127.0.0.1:5173`. Vite forwards `/api` and `/ws` to the Rust process on port 8080. The terminal running the server prints the table passphrase and the GM key. The GM types that key in the lobby. Players type only the passphrase.

## Run it for a game

```bash
cd client && npm install && npm run build
cargo build --release --manifest-path server/Cargo.toml
./server/target/release/battle-map
```

Start that binary from this directory. It serves `client/dist` and listens on `0.0.0.0:8080`. Copy `config.example.toml` to `config.toml` if you want a passphrase and GM key that survive a restart. An empty `gm_key` is filled in with a new key each launch and printed in the terminal.

Saves land in `data/state.json`, a moment after each change, and again when you stop the process with Ctrl-C. Map images and token portraits live under `data/` as well. An older save is copied beside itself before it is opened. The GM then sees what that open kept: the map names, a picture that was not rebuilt into tiles, a wall with no recorded height (treated as 8 units), and a portrait that is not yet a figure. To go back, stop the table and replace `data/state.json` with that `.bak-` copy. Do not replace it while the table is running.

## Friends on Tailscale

[Tailscale](https://tailscale.com) is a private network between your machines. It is not related to Tailwind, which is only the CSS library used for the screen.

1. Install Tailscale on the GM machine and on each player's machine, and sign them into the same tailnet.
2. On the GM machine run `tailscale ip -4` and note the address.
3. Start the game table so it binds `0.0.0.0:8080`.
4. Each player opens `http://<that-address>:8080`.

If a player's browser cannot connect, allow inbound TCP 8080 on the GM machine. People who do not have Tailscale installed can be let in later with `tailscale funnel`; do that only when you want the table reachable from the public internet, and keep the passphrase and GM key private.

## At the table

- Drag empty ground to pan. The wheel zooms. The camera angle is fixed.
- Drag a token to a square. The server snaps it to the grid and refuses a move through a wall or a closed door. Players can drag only their own tokens. The GM can drag any of them.
- Fog starts unexplored. A token with sight paints what it can see, and that memory stays. Explored ground is dim; what the token sees right now is clear. The GM sees the whole map under a veil, and can paint fog in or out. "See as" shows a player's view.
- Click a door, as the GM, to open or close it. A closed door blocks sight and movement.
- Build the map in the editor: paint the ground, place objects, and put it on the table. An angled picture already stored with a save stays filed. The table draws the fixed view. Fog hides that view. It does not stop a seat from downloading a picture the map still names.
- The dice hotbar builds a formula such as `2d6+3` or `1d20`. The server rolls them in a tray and sends every seat the same tumble, played at the same moment. **Everyone** shows the roll. **To the GM** shows it to the roller and the GM. **Secret** is GM-only; other seats get a short notice and never the number.

## Tests

```bash
cargo test --manifest-path server/Cargo.toml
cd client && npm test
```

`cargo test` also writes the TypeScript message types into `client/src/lib/net/bindings`. Do not edit those files; change the Rust types and export them again.
