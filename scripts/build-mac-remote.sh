#!/usr/bin/env bash
# Build CLAP + VST3 bundles for macOS on a remote Mac via SSH.
#
# The Mac is the only path for Apple targets (no cross-Apple SDK on Linux).
# This script:
#   1. pre-flight checks the SSH connection (BatchMode, 5 s timeout);
#   2. ensures Rust + the required targets are present on the Mac;
#   3. clones or updates the repo on the Mac;
#   4. runs `cargo xtask bundle` for aarch64 + x86_64 (universal build);
#   5. rsyncs the bundles back into dist/macos-universal/ on this host.
#
# Required env var:
#   RS_MAC_SSH_HOST   — SSH destination, e.g. "user@macbook.local" or just
#                       the host alias from ~/.ssh/config.  NEVER hard-coded.
#
# Optional env vars:
#   RS_MAC_REPO_PATH  — where to clone/pull on the Mac (default: ~/skyhub-v2)
#   RS_MAC_SSH_PORT   — SSH port (default: 22)
#   RS_REMOTE_REF     — git ref to build (default: HEAD of current branch)
#
# Prerequisites on the remote Mac:
#   - Xcode Command Line Tools: xcode-select --install
#   - rustup: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
#   - rustup targets: aarch64-apple-darwin + x86_64-apple-darwin
#   - SSH key authorized in ~/.ssh/authorized_keys on the Mac
#   - Tailscale up (if connecting over tailnet)
#
# Usage:
#   RS_MAC_SSH_HOST=mac-user@mymac.example bash scripts/build-mac-remote.sh
set -euo pipefail
cd "$(dirname "$0")/.."

: "${RS_MAC_SSH_HOST:?RS_MAC_SSH_HOST must be set (e.g. user@macbook.local)}"
REPO_PATH="${RS_MAC_REPO_PATH:-~/skyhub-v2}"
SSH_PORT="${RS_MAC_SSH_PORT:-22}"

# Determine the git remote URL from the local clone.
REPO_URL="$(git remote get-url origin 2>/dev/null || true)"
if [ -z "$REPO_URL" ]; then
  echo "ERROR: could not determine remote URL (no 'origin' remote)."
  echo "Either add a remote (git remote add origin <url>) or set RS_MAC_REPO_PATH"
  echo "to a path on the Mac where the repo is already cloned."
  exit 1
fi

REF="${RS_REMOTE_REF:-$(git rev-parse --abbrev-ref HEAD)}"
DIST=dist/macos-universal

SSH_OPTS=(-p "$SSH_PORT" -o BatchMode=yes -o StrictHostKeyChecking=accept-new)

echo "=== macOS remote build ==="
echo "  Host : $RS_MAC_SSH_HOST"
echo "  Path : $REPO_PATH"
echo "  Ref  : $REF"
echo ""

# 1. Pre-flight: can we reach the Mac?
echo "--- pre-flight SSH check ---"
if ! ssh "${SSH_OPTS[@]}" -o ConnectTimeout=5 "$RS_MAC_SSH_HOST" true 2>/dev/null; then
  echo ""
  echo "BLOCKED: cannot reach $RS_MAC_SSH_HOST"
  echo ""
  echo "To unblock:"
  echo "  1. Make sure the Mac is on and Tailscale is up."
  echo "  2. Add your SSH key: ssh-copy-id -p $SSH_PORT $RS_MAC_SSH_HOST"
  echo "  3. Set RS_MAC_SSH_HOST to the correct address, e.g.:"
  echo "       RS_MAC_SSH_HOST=youruser@your-mac-hostname RS_MAC_SSH_PORT=22 bash $0"
  echo ""
  echo "Pre-flight failed — nothing was built. No partial output."
  exit 1
fi
echo "  SSH reachable."

# 2. Ensure rustup + targets on the Mac.
echo "--- ensuring Rust toolchain on Mac ---"
ssh "${SSH_OPTS[@]}" "$RS_MAC_SSH_HOST" bash -s <<'REMOTE'
set -euo pipefail
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
if ! command -v rustup >/dev/null 2>&1; then
  echo "ERROR: rustup not found on Mac."
  echo "Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi
rustup target add aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null || true
echo "  Rust targets: $(rustup target list --installed | tr '\n' ' ')"
REMOTE

# 3. Clone or update the repo on the Mac.
echo "--- syncing repo on Mac ---"
ssh "${SSH_OPTS[@]}" "$RS_MAC_SSH_HOST" bash -s -- "$REPO_URL" "$REPO_PATH" "$REF" <<'REMOTE'
set -euo pipefail
REPO_URL="$1"; REPO_PATH="$2"; REF="$3"
REPO_PATH="${REPO_PATH/#\~/$HOME}"   # expand tilde
if [ -d "$REPO_PATH/.git" ]; then
  echo "  pulling $REPO_PATH..."
  git -C "$REPO_PATH" fetch --quiet origin
  git -C "$REPO_PATH" checkout --quiet "$REF" 2>/dev/null || \
    git -C "$REPO_PATH" checkout --quiet "origin/$REF"
  git -C "$REPO_PATH" pull --quiet --ff-only origin "$REF" 2>/dev/null || true
else
  echo "  cloning $REPO_URL..."
  git clone --quiet "$REPO_URL" "$REPO_PATH"
  git -C "$REPO_PATH" checkout --quiet "$REF" 2>/dev/null || true
fi
REMOTE

# 4. Build on Mac (aarch64 + x86_64 = two separate bundles, then manual lipo
#    for true universal binaries for the CLAP flat file).
echo "--- building plugins on Mac ---"
ssh "${SSH_OPTS[@]}" "$RS_MAC_SSH_HOST" bash -s -- "$REPO_PATH" <<'REMOTE'
set -euo pipefail
REPO_PATH="${1/#\~/$HOME}"
cd "$REPO_PATH"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"

PLUGINS=(plugin-morph plugin-phaseplan)

build_target() {
  local plugin="$1" target="$2"
  echo "  bundling $plugin ($target)..."
  cargo xtask bundle "$plugin" --release --target "$target"
}

# Build arm64 first
for plugin in "${PLUGINS[@]}"; do build_target "$plugin" aarch64-apple-darwin; done

mkdir -p target/bundled-arm64
for plugin in "${PLUGINS[@]}"; do
  cp -f  "target/bundled/${plugin}.clap"  "target/bundled-arm64/${plugin}.clap"
  cp -rTf "target/bundled/${plugin}.vst3" "target/bundled-arm64/${plugin}.vst3"
done

# Build x86_64
for plugin in "${PLUGINS[@]}"; do build_target "$plugin" x86_64-apple-darwin; done

mkdir -p target/bundled-x86_64
for plugin in "${PLUGINS[@]}"; do
  cp -f  "target/bundled/${plugin}.clap"  "target/bundled-x86_64/${plugin}.clap"
  cp -rTf "target/bundled/${plugin}.vst3" "target/bundled-x86_64/${plugin}.vst3"
done

# Universal CLAP (lipo the flat file).
mkdir -p target/bundled-universal
for plugin in "${PLUGINS[@]}"; do
  lipo -create \
    "target/bundled-arm64/${plugin}.clap" \
    "target/bundled-x86_64/${plugin}.clap" \
    -output "target/bundled-universal/${plugin}.clap"
done

# VST3: copy arm64 bundle, replace the binary with a lipo'd universal.
for plugin in "${PLUGINS[@]}"; do
  cp -rTf "target/bundled-arm64/${plugin}.vst3" "target/bundled-universal/${plugin}.vst3"
  ARM_BIN="target/bundled-arm64/${plugin}.vst3/Contents/MacOS/${plugin}"
  X86_BIN="target/bundled-x86_64/${plugin}.vst3/Contents/MacOS/${plugin}"
  UNI_BIN="target/bundled-universal/${plugin}.vst3/Contents/MacOS/${plugin}"
  if [ -f "$ARM_BIN" ] && [ -f "$X86_BIN" ]; then
    lipo -create "$ARM_BIN" "$X86_BIN" -output "$UNI_BIN"
  fi
done

echo ""
echo "Universal bundles:"
find target/bundled-universal -type f | sort | while read -r f; do
  printf "  %-60s  %s\n" "$f" "$(shasum -a 256 "$f" | cut -d' ' -f1)"
done
REMOTE

# 5. Retrieve artifacts.
echo "--- retrieving artifacts ---"
mkdir -p "$DIST"
rsync -av -e "ssh ${SSH_OPTS[*]}" \
  "${RS_MAC_SSH_HOST}:${REPO_PATH}/target/bundled-universal/" \
  "$DIST/"

echo ""
echo "macOS universal artifacts in $DIST:"
find "$DIST" -type f | sort | while read -r f; do
  printf "  %-60s  %s\n" "$f" "$(sha256sum "$f" | cut -d' ' -f1)"
done
echo ""
echo "NOTE: these bundles are unsigned (Gatekeeper will warn)."
echo "Right-click the plugin in Finder and choose Open to load it the first time."
echo "For distribution, code-sign with: codesign --deep --sign 'Developer ID' <bundle>"
