#!/usr/bin/env bash
# Cross-compile CLAP + VST3 bundles for Windows (x86_64-pc-windows-msvc)
# from any Linux host, using cargo-xwin (no Windows VM needed).
#
# Prerequisites (all user-space, no sudo):
#   cargo install cargo-xwin
#   rustup target add x86_64-pc-windows-msvc
#
# cargo-xwin auto-downloads the MSVC CRT + Windows SDK headers/libs on first
# run (~300 MB, cached in ~/.cache/cargo-xwin/).  lld-link is bundled with it.
#
# Outputs land in dist/windows-x86_64/.
# Re-runnable; outputs are replaced on success.
#
# Usage:
#   bash scripts/build-plugins-windows.sh
set -euo pipefail
cd "$(dirname "$0")/.."

# Rust toolchain
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"

TARGET=x86_64-pc-windows-msvc
DIST=dist/windows-x86_64

# Sanity checks
if ! command -v cargo-xwin >/dev/null 2>&1; then
  echo "ERROR: cargo-xwin not found."
  echo "Install with: cargo install cargo-xwin"
  exit 1
fi
if ! rustup target list --installed | grep -q "$TARGET"; then
  echo "ERROR: Rust target $TARGET not installed."
  echo "Install with: rustup target add $TARGET"
  exit 1
fi

mkdir -p "$DIST"

# Load the xwin linker/compiler env vars (lld-link, clang-cl, SDK paths).
# This makes xtask's inner cargo invocation find link.exe → lld-link.
eval "$(cargo-xwin env --target "$TARGET")"

PLUGINS=(plugin-botanica plugin-phaseplan)

for plugin in "${PLUGINS[@]}"; do
  echo "--- bundling $plugin (windows/$TARGET) ---"
  cargo xtask bundle "$plugin" --release --target "$TARGET"
done

# nih-plug writes CLAP as a flat file and VST3 with per-target sub-directories
# (x86_64-win/, x86_64-linux/, etc.) accumulated in the same tree.  Copy the
# Windows CLAP and only the x86_64-win sub-directory from the VST3 bundle so
# the dist/ Windows folder contains no Linux .so files.
for plugin in "${PLUGINS[@]}"; do
  cp -f "target/bundled/${plugin}.clap"  "$DIST/${plugin}.clap"

  WIN_VST3="$DIST/${plugin}.vst3"
  mkdir -p "$WIN_VST3/Contents/x86_64-win"
  cp -f "target/bundled/${plugin}.vst3/Contents/x86_64-win/${plugin}.vst3" \
        "$WIN_VST3/Contents/x86_64-win/${plugin}.vst3"
done

echo ""
echo "Windows artifacts in $DIST:"
find "$DIST" -type f | sort | while read -r f; do
  printf "  %-60s  %s\n" "$f" "$(sha256sum "$f" | cut -d' ' -f1)"
done
