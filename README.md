# RanchSamples

A community-owned, self-hostable audio-samples platform — browse, preview, organize,
and drag straight into your DAW — with two native instruments (web + VST3/CLAP) built on
a single shared DSP core.

This is the **v2 re-foundation**: a clean monorepo with one source of truth per concern.
It supersedes the v1 prototype, keeping what was proven (the audio analyzer, the catalog
schema, the auth model, the drag research) and rebuilding what had accumulated cruft (a
duplicated backend, a monolithic frontend, and a forked DSP).

## Surfaces

- **Web app** (`apps/web`) — SvelteKit compiled to static files (no Node runtime in prod,
  CSP-strict). Browse / search / preview / organize into collections, community board,
  the studio (instruments running in an AudioWorklet via WebAssembly).
- **API** (`services/api`) — a single FastAPI service over a SQLite catalog.
- **Worker** (`services/worker`) — Python/librosa audio analysis (BPM, key, timbre) + ingest.
- **DSP core** (`crates/dsp-core`) — real-time-safe Rust primitives, compiled **once** to
  both WebAssembly (the web studio) and native plugins (VST3/CLAP via `nih-plug`).
- **Instruments** (`crates/botanica`, `crates/phaseplan`) — built on `dsp-core`.
- **Desktop** (`crates/desktop`) — a Tauri shell whose reason to exist is reliable
  drag-into-DAW (a real local library folder + a native-drag enhancement layer).

## Principles

- **One source of truth.** The DSP is one Rust core, not a JS copy and a Rust copy. The
  backend is one server, not two kept in lockstep.
- **Accessibility first.** Calm UI, high legibility (Lexend / Atkinson Hyperlegible), warm
  near-black, AA+ contrast, `prefers-reduced-motion` honoured, no alarm iconography.
- **Self-hostable & auditable.** Static frontend, single SQLite file, reproducible builds,
  artifact residue audits. No SaaS dependency.
- **Original work.** The instruments are *inspired by* well-known tools but contain no
  third-party code or assets. Sound-design credit: **Tev**. Meters: **polarity** (MIT).

## Layout

```
crates/      Rust workspace: dsp-core, instruments, wasm bindings, plugins, desktop, xtask
apps/web/    SvelteKit (adapter-static) frontend
services/    api (FastAPI) + worker (Python analyzer/ingest)
packages/    tokens (design system) + api-types (generated)
db/          canonical schema + forward-only migrations
tests/       api integration, dsp golden-render, e2e smoke
ops/ ci/     deployment units + build/audit pipeline
docs/        architecture, ADRs, security, operations
```

## Build

A single task runner (`just`) fronts every toolchain:

```sh
just setup     # install/sync toolchains (rust, node, python)
just dev       # run api + web in watch mode
just test      # cargo test + pytest + golden renders
just build     # web (static) + plugins + desktop bundles
just audit     # residue scan on built artifacts
```

## License

GNU AGPL-3.0-or-later. Network use is distribution: a hosted instance must offer its
source. See [`LICENSE`](LICENSE).
