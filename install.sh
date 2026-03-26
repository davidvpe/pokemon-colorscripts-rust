#!/bin/sh
set -e

BIN_DIR='/usr/local/bin'

# Detect repo from git remote so the script works for any fork
REPO=$(git remote get-url origin 2>/dev/null \
    | sed 's|https://github.com/||; s|git@github.com:||; s|\.git$||') || true

if [ -z "$REPO" ]; then
    echo "Error: could not detect GitHub repo. Run from inside the cloned repository."
    exit 1
fi

# Detect OS and pick the right binary
case "$(uname -s)" in
    Darwin) BINARY="pokemon-colorscripts-macos-universal" ;;
    Linux)  BINARY="pokemon-colorscripts-linux-x86_64" ;;
    *)
        echo "Unsupported OS: $(uname -s)"
        echo "For Windows use install.ps1"
        exit 1
        ;;
esac

URL="https://github.com/$REPO/releases/latest/download/$BINARY"

echo "Downloading $BINARY from $REPO..."

if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$URL" -o "$BIN_DIR/pokemon-colorscripts"
elif command -v wget >/dev/null 2>&1; then
    wget -q "$URL" -O "$BIN_DIR/pokemon-colorscripts"
else
    echo "Error: curl or wget is required."
    exit 1
fi

chmod +x "$BIN_DIR/pokemon-colorscripts"
echo "Installed to $BIN_DIR/pokemon-colorscripts"
