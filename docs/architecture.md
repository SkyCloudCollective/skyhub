# Architecture

SkyHub v2 is a monorepo with **one source of truth per concern**. This
document is the map; ADR-style decisions follow.

## Surfaces & data flow

```
                ┌─────────────────────────────────────────────┐
  Browser  ───► │  apps/web  (SvelteKit static SPA, CSP-strict)│
  / Desktop     └───────────────┬─────────────────────────────┘
  (Tauri shell)                 │  fetch /v1/* (connect-src)
                                ▼
                 ┌──────────────────────────────┐      ┌───────────────────┐
                 │  services/api (FastAPI)       │◄────►│  db (SQLite, WAL) │
                 └───────────────┬──────────────┘      └───────────────────┘
                                 │ enqueue / subprocess
                                 ▼
                 ┌──────────────────────────────┐
                 │ services/worker (librosa)     │  BPM · key · timbre · ingest
                 └──────────────────────────────┘

  Audio engine (one Rust core, two frontends):
     crates/dsp-core ─► crates/{morph,phaseplan}
                          ├─► crates/dsp-wasm     ─► apps/web studio (AudioWorklet)
                          └─► crates/plugin-*      ─► VST3 / CLAP (nih-plug)
```

## Decisions (ADR digest)

- **ADR-1 — One backend (FastAPI), not two.** v1 maintained a stdlib server and
  a FastAPI server in lockstep via a parity test. v2 keeps only FastAPI: less
  code, OpenAPI docs for free, types generated for the frontend.
- **ADR-2 — Static SPA frontend (SvelteKit + adapter-static).** Compiles to
  plain files served by Caddy (no Node runtime in prod). CSP stays `script-src
  'self'`. Components + router + one keyed i18n dict replace the v1 monolith.
- **ADR-3 — One Rust DSP core.** `dsp-core` holds every primitive; instrument
  crates depend on it; wasm and plugin wrappers are thin and contain no DSP, so
  web and native run byte-identical code. Proven by golden-render tests
  (`tests/dsp/golden`) that compare wasm vs native output.
- **ADR-4 — Favourites are a system collection.** No `favorites` table; a
  per-user `collections.kind='favorites'` row, so "favourite" and "organise into
  crates" share one model, one UI, one share mechanism.
- **ADR-5 — Desktop drag is layered.** Reliable layer: downloads route to a
  local library folder (Tauri `on_download`, no remote IPC permission needed);
  drag from the file manager always works. Enhancement layer: native in-app drag
  (`tauri-plugin-drag`) with pre-cache. The app is usable even if native drag
  misbehaves on a given OS/DAW.
- **ADR-6 — Identity = Matrix handle.** No bespoke accounts; auth is dormant
  until a homeserver is configured (open dev mode otherwise).

## Real-time DSP contract

Anything reachable from an audio callback must not allocate, lock, do I/O, or
panic; all state is sized once; parameters are smoothed. `dsp-core` is
`#![forbid(unsafe_code)]`. See `crates/dsp-core/src/lib.rs`.

## Build & audit

`just` fronts every toolchain. CI runs four jobs: `rust` (fmt + clippy + test),
`api` (pytest), `web` (check + static build), and `audit` (residue scan).
Reproducible builds + the residue gate (`ci/audit-artifact.py`) precede any
publish. macOS builds run on real Apple hardware / CI macOS runners; Linux and
Windows builds + headless plugin validation (pluginval) run in
containers/VMs on the homelab.

## Frontiers (operational)

Agent does: all code, tests, local builds, local git. Operator applies: real
secrets, DNS/exposure, member provisioning, plugin signing, repo exposure.
Never: public user-generated content before the legal pass; sending credentials
or messages to members; committing real secrets; unsolicited deployment.
