#!/usr/bin/env bash
# Orchestrate all plugin builds for reachable platforms, then produce a
# versioned dist/ set with a sha256 manifest.
#
# Platforms attempted (those unavailable are skipped with a clear message):
#   linux-x86_64     — always built on this host
#   windows-x86_64   — cross-compiled via cargo-xwin (if installed)
#   macos-universal  — remote Mac build (if RS_MAC_SSH_HOST is set and reachable)
#
# Usage:
#   bash scripts/build-all.sh
#   RS_MAC_SSH_HOST=user@mymac bash scripts/build-all.sh   # include macOS
#
# Outputs:
#   dist/<platform>/plugin-botanica.{clap,vst3}
#   dist/<platform>/plugin-phaseplan.{clap,vst3}
#   dist/SHA256SUMS                  — all artifact hashes, one per line
#   dist/manifest.json               — machine-readable per-platform manifest
#
# Version stamp: reads crates version from Cargo.toml [workspace.package].
# Set BUILD_VERSION env to override (useful in CI).
set -euo pipefail
cd "$(dirname "$0")/.."

[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"

SCRIPTS="$(pwd)/scripts"

# Version from workspace Cargo.toml, or override.
VERSION="${BUILD_VERSION:-$(grep -m1 '^version\s*=' Cargo.toml | grep -oP '[\d]+\.[\d]+\.[\d]+' || echo "0.0.0")}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

echo "======================================================"
echo " SkyHub plugin build — all platforms"
echo " version : $VERSION"
echo " time    : $TIMESTAMP"
echo "======================================================"
echo ""

mkdir -p dist

BUILT=()
SKIPPED=()

# ── Linux ──────────────────────────────────────────────────────────────────
echo "[1/3] Linux (x86_64)"
bash "$SCRIPTS/build-plugins-linux.sh" --release
BUILT+=("linux-x86_64")
echo ""

# ── Windows ────────────────────────────────────────────────────────────────
echo "[2/3] Windows (x86_64, cross via cargo-xwin)"
if command -v cargo-xwin >/dev/null 2>&1 && \
   rustup target list --installed | grep -q x86_64-pc-windows-msvc; then
  bash "$SCRIPTS/build-plugins-windows.sh"
  BUILT+=("windows-x86_64")
else
  echo "  SKIPPED — cargo-xwin or x86_64-pc-windows-msvc target not installed."
  echo "  To enable:"
  echo "    cargo install cargo-xwin"
  echo "    rustup target add x86_64-pc-windows-msvc"
  SKIPPED+=("windows-x86_64")
fi
echo ""

# ── macOS ──────────────────────────────────────────────────────────────────
echo "[3/3] macOS (universal, remote Mac via SSH)"
if [ -n "${RS_MAC_SSH_HOST:-}" ]; then
  bash "$SCRIPTS/build-mac-remote.sh"
  BUILT+=("macos-universal")
else
  echo "  SKIPPED — RS_MAC_SSH_HOST not set."
  echo "  To enable:"
  echo "    RS_MAC_SSH_HOST=user@your-mac bash scripts/build-all.sh"
  SKIPPED+=("macos-universal")
fi
echo ""

# ── Manifest + SHA256SUMS ──────────────────────────────────────────────────
echo "--- generating dist/SHA256SUMS ---"
SUMS_FILE=dist/SHA256SUMS
: > "$SUMS_FILE"   # truncate
find dist -type f ! -name SHA256SUMS ! -name manifest.json | sort | while read -r f; do
  sha256sum "$f" >> "$SUMS_FILE"
done
cat "$SUMS_FILE"
echo ""

echo "--- generating dist/manifest.json ---"
python3 - "$VERSION" "$TIMESTAMP" <<'PYEOF'
import json, pathlib, subprocess, sys

version   = sys.argv[1]
timestamp = sys.argv[2]
dist      = pathlib.Path("dist")

platforms = {}
for platform_dir in sorted(dist.iterdir()):
    if not platform_dir.is_dir():
        continue
    name = platform_dir.name
    entries = {}
    for f in sorted(platform_dir.rglob("*")):
        if f.is_file():
            digest = subprocess.check_output(["sha256sum", str(f)]).split()[0].decode()
            entries[str(f.relative_to(dist))] = {
                "size": f.stat().st_size,
                "sha256": digest,
            }
    platforms[name] = entries

manifest = {
    "version": version,
    "built_at": timestamp,
    "platforms": platforms,
}
pathlib.Path("dist/manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
print("  written dist/manifest.json")
PYEOF

# ── Residue audit on built artifacts ──────────────────────────────────────
echo "--- residue audit (built artifacts) ---"
PATTERNS_ARG=""
[ -f ci/audit-patterns.local ] && PATTERNS_ARG="--patterns ci/audit-patterns.local"
python3 ci/audit-artifact.py dist $PATTERNS_ARG || {
  echo "WARNING: residue detected in built artifacts — see above."
  echo "Resolve before distributing."
}
echo ""

# ── Summary ────────────────────────────────────────────────────────────────
echo "======================================================"
echo " Build complete"
echo " version : $VERSION  ($TIMESTAMP)"
echo " built   : ${BUILT[*]:-none}"
echo " skipped : ${SKIPPED[*]:-none}"
echo ""
echo " dist layout:"
find dist -type f | sort | while read -r f; do
  sz=$(stat -c%s "$f" 2>/dev/null || stat -f%z "$f" 2>/dev/null || echo "?")
  printf "  %-58s  %7d bytes\n" "$f" "$sz"
done
echo "======================================================"
