#!/usr/bin/env bash
# ------------------------------------------------------------
# Script: run-linux.sh
# Purpose: Start the Rust server and the Vite client so the
#          table can be opened locally.
# Usage:   bash scripts/run-linux.sh
#          chmod +x scripts/run-linux.sh && ./scripts/run-linux.sh
# ------------------------------------------------------------
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo was not found. Install Rust from https://rustup.rs and open a new terminal."
  exit 1
fi
if ! command -v npm >/dev/null 2>&1; then
  echo "npm was not found. Install Node.js from https://nodejs.org and open a new terminal."
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
server_cmd="$(printf 'cd %q && cargo run --manifest-path server/Cargo.toml' "$repo_root")"
client_cmd="$(printf 'cd %q && npm install && npm run dev' "$repo_root/client")"

# ------------------------------------------------------------
# Function: open_terminal
# Purpose: Open one command in a new terminal window when a
#          desktop terminal is available.
# ------------------------------------------------------------
open_terminal() {
  local title="$1"
  local command="$2"
  if command -v gnome-terminal >/dev/null 2>&1; then
    gnome-terminal --title="$title" -- bash -lc "$command; exec bash"
  elif command -v konsole >/dev/null 2>&1; then
    konsole --new-tab -p tabtitle="$title" -e bash -lc "$command; exec bash"
  elif command -v x-terminal-emulator >/dev/null 2>&1; then
    x-terminal-emulator -T "$title" -e bash -lc "$command; exec bash" &
  elif command -v xterm >/dev/null 2>&1; then
    xterm -T "$title" -e bash -lc "$command; exec bash" &
  else
    return 1
  fi
}

if open_terminal "Longtable server" "$server_cmd" && open_terminal "Longtable client" "$client_cmd"; then
  echo "Server and client started in new terminals."
  echo "The server terminal prints the table passphrase and the GM key."
  echo "Open http://127.0.0.1:5173 when the client is ready."
  echo "Close both terminals to stop the table."
  exit 0
fi

echo "Starting the server in the background and the client in this window."
echo "The server prints the table passphrase and the GM key."
echo "Open http://127.0.0.1:5173 when the client is ready."
echo "Press Ctrl-C to stop both."

cd "$repo_root"
cargo run --manifest-path server/Cargo.toml &
server_pid=$!

cleanup() {
  kill "$server_pid" 2>/dev/null || true
  wait "$server_pid" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

cd "$repo_root/client"
npm install
npm run dev
