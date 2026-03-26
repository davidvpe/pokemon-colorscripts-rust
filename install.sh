#!/bin/sh
set -e

REPO="davidvpe/pokemon-colorscripts-rust"
BIN_DIR="/usr/local/bin"

case "$(uname -s)" in
    Darwin) BINARY="pokemon-colorscripts-macos-universal" ;;
    Linux)  BINARY="pokemon-colorscripts-linux-x86_64" ;;
    *)
        echo "Unsupported OS. For Windows run install.ps1"
        exit 1
        ;;
esac

URL="https://github.com/$REPO/releases/latest/download/$BINARY"

echo "Installing pokemon-colorscripts..."

if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$BIN_DIR/pokemon-colorscripts"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$URL" -O "$BIN_DIR/pokemon-colorscripts"
else
    echo "Error: curl or wget is required."
    exit 1
fi

chmod +x "$BIN_DIR/pokemon-colorscripts"
echo "Done! Run: pokemon-colorscripts --random"
