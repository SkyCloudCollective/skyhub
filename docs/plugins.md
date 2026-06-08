# Native plugins (CLAP / VST3)

Both instruments ship as real DAW plugins via [nih-plug](https://github.com/robbert-vdh/nih-plug).
The plugin crates are **thin wrappers** — they own no DSP; they call the same
`phaseplan` / `morph` engines the web studio runs through wasm, so the plugin
and the browser produce byte-identical synthesis.

- `crates/plugin-phaseplan` — **PhasePlan**, a MIDI instrument (CLAP + VST3).
- `crates/plugin-morph` — **Morph**, a generative instrument (CLAP + VST3),
  no MIDI; steered by the Character XY + macros. Sound design by **Tev**.

## Build

```bash
. "$HOME/.cargo/env"
cargo xtask bundle plugin-phaseplan --release
cargo xtask bundle plugin-morph  --release
# → target/bundled/{plugin-phaseplan,plugin-morph}.{clap,vst3}
```

`cargo xtask` is an alias (`.cargo/config.toml`) for the `xtask` crate, which wraps
`nih_plug_xtask` to produce platform-correct `.clap` files and `.vst3` bundle
directories. The bundles are build artifacts (under the gitignored `target/`).

### Dependencies / portability

`nih_plug` is pinned by git rev with **default features only** (`vst3`). We do
**not** enable the `standalone` feature, which would pull in JACK/ALSA/cpal — none
of which are installed here, and none of which the CLAP/VST3 *export* needs. The
build is pure-Rust (`vst3-sys`, `clap-sys`) and needs no system audio libraries on
Linux. macOS/Windows builds + code signing/notarisation run in CI / on Marwan's
Mac (JAUNE), as does `pluginval`.

## Verification done

- `nm -D` on the built `.so`/`.clap` shows the entry points: `clap_entry` (CLAP)
  and `GetPluginFactory` + `ModuleEntry` (VST3).
- `cargo clippy -D warnings` clean on both wrappers; the engines are tested in
  their own crates.

## TODO (CI / JAUNE)

- `pluginval --strictness-level 8` on the VST3 in CI (binary not installed locally).
- Golden-render test asserting **wasm == native** (same params + note sequence →
  identical buffer) — the structural anti-divergence guarantee.
- macOS (arm64 + intel) and Windows bundles via the GitHub Actions matrix; signing
  + notarisation with Marwan's keys.
