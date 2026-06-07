#!/usr/bin/env bash
# Build CLAP + VST3 bundles for Linux (x86_64) on the current host.
#
# Outputs land in dist/linux-x86_64/ (CLAP files + VST3 bundle dirs).
# Re-runnable; earlier outputs are replaced on success.
#
# Usage:
#   bash scripts/build-plugins-linux.sh [--release]          (default: release)
#   bash scripts/build-plugins-linux.sh [--dev]              (skip release flag)
set -euo pipefail
cd "$(dirname "$0")/.."

PROFILE="--release"
for arg in "$@"; do
  case "$arg" in
    --dev)   PROFILE="" ;;
    --release) PROFILE="--release" ;;
  esac
done

# Rust toolchain
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"

DIST=dist/linux-x86_64
mkdir -p "$DIST"

PLUGINS=(plugin-botanica plugin-phaseplan)

for plugin in "${PLUGINS[@]}"; do
  echo "--- bundling $plugin (linux) ---"
  cargo xtask bundle "$plugin" $PROFILE
done

# nih-plug accumulates all targets in one target/bundled/ tree.  For Linux
# dist/ we only want the Linux CLAP (already platform-native) and the
# x86_64-linux sub-directory of the VST3 bundle (not any Windows entries).
for plugin in "${PLUGINS[@]}"; do
  cp -f "target/bundled/${plugin}.clap"  "$DIST/${plugin}.clap"

  LINUX_VST3="$DIST/${plugin}.vst3"
  mkdir -p "$LINUX_VST3/Contents/x86_64-linux"
  cp -f "target/bundled/${plugin}.vst3/Contents/x86_64-linux/${plugin}.so" \
        "$LINUX_VST3/Contents/x86_64-linux/${plugin}.so"
done

echo ""
echo "Linux artifacts in $DIST:"
find "$DIST" -type f | sort | while read -r f; do
  printf "  %-60s  %s\n" "$f" "$(sha256sum "$f" | cut -d' ' -f1)"
done
