#!/bin/sh

# Install script for pokemon-colorscripts (Rust)

BIN_DIR='/usr/local/bin'

# Build only the main binary (generator deps are excluded)
cargo build --release --bin pokemon-colorscripts || exit 1

# Install binary
cp target/release/pokemon-colorscripts "$BIN_DIR/pokemon-colorscripts" || exit 1

echo "Installed pokemon-colorscripts to $BIN_DIR/pokemon-colorscripts"
