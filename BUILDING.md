# Building RanchSamples

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
- `dist/linux-x86_64/plugin-botanica.clap`
- `dist/linux-x86_64/plugin-botanica.vst3/Contents/x86_64-linux/plugin-botanica.so`
- same for plugin-phaseplan

Install paths for testing in a DAW:
- CLAP: `~/.clap/`
- VST3: `~/.vst3/`

### Windows x86_64 — cross from Linux

```
bash scripts/build-plugins-windows.sh
```

Outputs:
- `dist/windows-x86_64/plugin-botanica.clap`
- `dist/windows-x86_64/plugin-botanica.vst3/Contents/x86_64-win/plugin-botanica.vst3`
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
- `RS_MAC_REPO_PATH` — path on the Mac where the repo is cloned (default: `~/ranchsamples-v2`)
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
cargo build -p plugin-botanica --release

# Bundle (CLAP + VST3, goes to target/bundled/):
cargo xtask bundle plugin-botanica --release
cargo xtask bundle plugin-phaseplan --release

# Windows bundle (requires xwin env):
eval "$(cargo-xwin env --target x86_64-pc-windows-msvc)"
cargo xtask bundle plugin-botanica --release --target x86_64-pc-windows-msvc
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

The `crates/desktop` crate is scaffolded but not yet wired into the workspace
(the `Cargo.toml` member line is commented out). When it lands:

- Linux/Windows: `cargo tauri build` (requires `npm`/`pnpm` for the front-end).
- macOS: same remote-Mac flow as the plugins.
- Cross-compilation for Tauri is not supported upstream; each OS must build
  its own native binary.

No sudo is needed for the Tauri build itself. The AppImage/deb/rpm/msi
packager steps may need system packages — those will be documented when the
desktop crate is activated.
