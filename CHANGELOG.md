# Changelog

Notable changes to SkyHub. The format follows
[Keep a Changelog](https://keepachangelog.com/); the project aims at
[Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-06-08

### Fixed

- **Desktop app now builds.** Committed a proper square app-icon set under
  `crates/desktop/icons/` (generated from `crates/desktop/app-icon.png`) instead of
  generating icons at build time from a non-square screenshot — which failed the Tauri
  Linux and Windows jobs in v0.1.0.
- **Container image now publishes to GHCR.** The image path is lower-cased
  (`ghcr.io/skycloudcollective/skyhub`), as the registry requires.

### Added

- First desktop bundles (Linux `.AppImage` + `.deb`, Windows NSIS `.exe`) and the GHCR
  container image, alongside the plugins.

## [0.1.0] - 2026-06-08

The first tagged release — the v2 re-foundation: one monorepo, one source of truth per concern.
The plugin bundles (CLAP + VST3: Linux, Windows, macOS arm64) ship here; the desktop bundles
and container image follow in 0.1.1.

### Added

- **DSP core** (`crates/dsp-core`) compiled once to WebAssembly (web studio) and native
  plugins (VST3/CLAP via `nih-plug`); two instruments (Morph, PhasePlan).
- **Web app** (`apps/web`, SvelteKit static): library browse/search/preview, drag-into-DAW,
  the studio (instruments in an AudioWorklet), the Galaxy timbre map, a media hub, the community board.
- **API** (`services/api`, FastAPI over SQLite) + **worker** (librosa BPM/key/timbre analysis).
- **Desktop shell** (`crates/desktop`, Tauri) for reliable drag-into-DAW; CI builds an
  AppImage + `.deb` (Linux) and an NSIS installer (Windows) on a `v*` tag.
- Per-member theming: six palettes, a custom accent, light/dark, saved themes.
- Self-host path: a single FastAPI process + one SQLite file + a prebuilt static app
  (`SELFHOST.md`), plus a multi-stage container image.
- Project docs: `SECURITY.md`, `.github/FUNDING.yml`, contributor + governance docs.

### Changed

- The theme-customisation demo is a lossless animated WebP (full colour) instead of a GIF.
