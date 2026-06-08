# Changelog

Notable changes to SkyHub. The format follows
[Keep a Changelog](https://keepachangelog.com/); the project aims at
[Semantic Versioning](https://semver.org/).

## [Unreleased]

The v2 re-foundation — one monorepo, one source of truth per concern.

### Added

- **DSP core** (`crates/dsp-core`) compiled once to WebAssembly (web studio) and native
  plugins (VST3/CLAP via `nih-plug`); two instruments (Botanica, PhasePlan).
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

_`v0.1.0` will be the first tagged release — it triggers the plugin + desktop + container build._
