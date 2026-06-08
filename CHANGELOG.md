# Changelog

Notable changes to SkyHub. The format follows
[Keep a Changelog](https://keepachangelog.com/); the project aims at
[Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-06-08

### Added

- **Container image** published to GHCR (`ghcr.io/skycloudcollective/skyhub`),
  alongside the plugin bundles.

### Fixed

- **GHCR push** now uses a lower-cased image path, which the registry requires.
- **Release resilience**: the release publishes the plugin bundles (+ container)
  without being held hostage by the best-effort desktop jobs, and pulls artifacts by
  name instead of grabbing the auto-generated docker build-record (which broke publish).
- Committed a proper square app-icon set under `crates/desktop/icons/` (from
  `crates/desktop/app-icon.png`) for the desktop build.

### Notes

- Desktop bundles (Tauri AppImage / deb / NSIS) are a tracked follow-up: the build job
  runs on manual dispatch while it's debugged and does not gate the release.

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
