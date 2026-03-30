#!/usr/bin/env bash
# Usage: ./scripts/test.sh <pokemon>
# Deletes the colorscript and stat for the given pokemon, regenerates it, then displays all 3 modes.
set -e
cd "$(dirname "$0")/.."

name="${1:?Usage: $0 <pokemon>}"

cargo run --bin generator --features generator -- "$name"

echo "=== simple ==="
cargo run --bin pokemon-colorscripts -- --name "$name"
echo "=== info ==="
cargo run --bin pokemon-colorscripts -- --name "$name" --info
echo "=== pokedex ==="
cargo run --bin pokemon-colorscripts -- --name "$name" --pokedex
