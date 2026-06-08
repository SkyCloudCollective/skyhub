# Building SkyHub

TL;DR — one command per platform:

| Platform | Command |
|---|---|
| Linux (current host) | `bash scripts/build-plugins-linux.sh` |
| Windows (cross from Linux) | `bash scripts/build-plugins-windows.sh` |
| macOS (remote Mac via SSH) | `RS_MAC_SSH_HOST=user@mac bash scripts/build-mac-remote.sh` |
| All platforms at once | `bash scripts/build-all.sh` |

Outputs land in `dist/<platform>/`. A `dist/SHA256SUMS` and `dist/manifest.json`
are written by `build-all.sh`.

---

## Prerequisites

### All platforms

Rust (stable), installed via rustup:

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After install, activate the toolchain in the current shell:

```
. "$HOME/.cargo/env"
```

### Linux

No extra system packages are required for CLAP and VST3 builds — nih-plug's
`vst3` feature does not pull in JACK or ALSA.

```
# Add the wasm target if you also want to build the web studio:
rustup target add wasm32-unknown-unknown
```

### Windows (cross-compilation from Linux)

All tooling is user-space; no sudo required.

```
cargo install cargo-xwin
rustup target add x86_64-pc-windows-msvc
```

On first run, `cargo-xwin` downloads the MSVC CRT and Windows SDK automatically
(~300 MB, cached in `~/.cache/cargo-xwin/`).  `lld-link` is bundled with it.

### macOS (remote build)

Required on the remote Mac:

1. Xcode Command Line Tools:
   ```
   xcode-select --install
   ```
2. rustup (same curl command as above).
3. Both Apple targets:
   ```
   rustup target add aarch64-apple-darwin x86_64-apple-darwin
   ```
4. The Mac's public key must be in the authorized_keys of the SSH user.
5. Tailscale up (if connecting over a tailnet).

---

## Build commands

### Linux x86_64 — native

```
bash scripts/build-plugins-linux.sh
```

Outputs:
- `dist/linux-x86_64/plugin-morph.clap`
- `dist/linux-x86_64/plugin-morph.vst3/Contents/x86_64-linux/plugin-morph.so`
- same for plugin-phaseplan

Install paths for testing in a DAW:
- CLAP: `~/.clap/`
- VST3: `~/.vst3/`

### Windows x86_64 — cross from Linux

```
bash scripts/build-plugins-windows.sh
```

Outputs:
- `dist/windows-x86_64/plugin-morph.clap`
- `dist/windows-x86_64/plugin-morph.vst3/Contents/x86_64-win/plugin-morph.vst3`
- same for plugin-phaseplan

These are unsigned binaries. On a Windows host, SmartScreen will warn.
Right-click the installer/binary and choose "Run anyway" (or load directly in
a CLAP/VST3 host that bypasses SmartScreen).

For signed distribution, Marwan's Windows code-signing certificate is required
(`WINDOWS_SIGN_CERT` / `WINDOWS_SIGN_KEY` — never stored in this repo).

### macOS universal — remote Mac via SSH

```
RS_MAC_SSH_HOST=youruser@your-mac-hostname bash scripts/build-mac-remote.sh
```

Optional variables:
- `RS_MAC_REPO_PATH` — path on the Mac where the repo is cloned (default: `~/skyhub-v2`)
- `RS_MAC_SSH_PORT` — SSH port (default: 22)
- `RS_REMOTE_REF` — git branch or tag to build (default: current branch)

The script:
1. Pre-flight checks SSH connectivity (5 s timeout, fails cleanly if unreachable).
2. Clones or `git pull`s the repo on the Mac.
3. Builds for `aarch64-apple-darwin` and `x86_64-apple-darwin`.
4. Creates universal binaries with `lipo`.
5. `rsync`s the result back into `dist/macos-universal/`.

The Mac must have the repo accessible — either cloned from a public remote or
via a shared git remote that both the Mac and this host can reach.

These are unsigned bundles. Gatekeeper will block them on first launch.
Workaround: right-click the plugin bundle in Finder, choose Open, click Open.

For notarized distribution, `MAC_SIGN_ID` (Marwan's Apple Developer certificate)
and `xcrun notarytool` are required — handled in the release CI workflow.

### All platforms

```
bash scripts/build-all.sh
# or, with macOS:
RS_MAC_SSH_HOST=youruser@your-mac bash scripts/build-all.sh
```

Produces `dist/SHA256SUMS` and `dist/manifest.json` in addition to the
per-platform bundles.

---

## Single-crate commands (faster for iteration)

```
# Build (no bundle):
cargo build -p plugin-morph --release

# Bundle (CLAP + VST3, goes to target/bundled/):
cargo xtask bundle plugin-morph --release
cargo xtask bundle plugin-phaseplan --release

# Windows bundle (requires xwin env):
eval "$(cargo-xwin env --target x86_64-pc-windows-msvc)"
cargo xtask bundle plugin-morph --release --target x86_64-pc-windows-msvc
```

---

## Residue scan

Before distributing any artifact, run the residue gate on the dist/ tree:

```
python3 ci/audit-artifact.py dist
# With deployment-specific patterns (gitignored, never committed):
python3 ci/audit-artifact.py dist --patterns ci/audit-patterns.local
```

Exit 0 = clean. Exit 1 = something internal leaked into an artifact.

---

## Tauri desktop app

`crates/desktop` is now an active workspace member. It wraps the SvelteKit
`apps/web/` build in a Tauri v2 shell with native drag-to-DAW support via
`tauri-plugin-drag`.

### Prerequisites

In addition to Rust (stable) and pnpm/Node 22:

**Linux**
```
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```
(Ubuntu 22.04+ / Debian Bookworm. Fedora: `webkit2gtk4.1-devel` etc.)

**Windows** — WebView2 ships with Windows 10/11; no extra install needed.

**macOS** — Xcode Command Line Tools (best-effort, not required for Linux/Windows CI).

### Icon generation (one-time, per checkout)

Icons are not committed to git. Generate them from the existing screenshot:
```
npx @tauri-apps/cli icon docs/screenshots/library-light.png \
  --output crates/desktop/icons
```
This produces `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.ico`, `icon.icns`
inside `crates/desktop/icons/`. Replace the screenshot with a proper branded
1024×1024 source when available.

### Local build

```
# Build the web front-end first:
cd apps/web && pnpm build && cd ../..

# Then build the Tauri app for the current OS:
cargo tauri build --project-path crates/desktop
```

Or with `just`:
```
just build-desktop
```

Outputs:
- Linux:   `target/release/bundle/appimage/SkyHub_*.AppImage`
           `target/release/bundle/deb/SkyHub_*_amd64.deb`
- Windows: `target/release/bundle/nsis/SkyHub_*-setup.exe`
- macOS:   `target/release/bundle/macos/SkyHub.app` (best-effort)

### Drag-to-DAW

The desktop shell integrates `tauri-plugin-drag` so that the SvelteKit front-end
can initiate a native OS drag session from a sample card directly into a DAW.
From the web layer, invoke:

```js
import { invoke } from "@tauri-apps/api/core";
await invoke("plugin:drag|drag_files", {
  items: [{ path: "/absolute/path/to/sample.wav", icon: null }],
  image: null,
});
```

Tauri's built-in drag-drop interception is disabled (`"dragDropEnabled": false`
in `tauri.conf.json`) so that outbound plugin-drag is not interfered with.

### Unsigned build caveats (JAUNE)

- **Linux**: no OS gating.
- **Windows**: SmartScreen warns on unsigned `.exe`. Right-click → "Run anyway".
- **macOS**: Gatekeeper blocks unsigned `.app`. Right-click → Open → Open.

Code signing (Apple Developer ID + `MAC_SIGN_ID`, Windows Authenticode +
`TAURI_SIGNING_PRIVATE_KEY`) is a planned operator follow-up. The keys are
Marwan's and are never stored in this repository.
