# Contributing

Thanks for helping build SkyHub with **the SkyCloudCollective**. Music, code,
docs, bug reports, and good ideas are all contributions. This guide covers the code
path; for how decisions are made see [`GOVERNANCE.md`](GOVERNANCE.md), and please read
the [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) first.

## Where to start

- **An idea or a feature?** Open it on the in-app **Board** (`/board`) so the whole
  collective can see and upvote it, or start a repo discussion. Proposing before
  building avoids wasted work and keeps the roadmap honest.
- **A bug?** A repo issue (or a Board entry) with steps to reproduce.
- **Code?** Read this file, then send a focused change.

## Set up

The repo is a Cargo workspace (Rust) + a pnpm workspace (web) + a Python service.

```sh
# Rust (DSP core, instruments, plugins)
. "$HOME/.cargo/env"
cargo test --workspace

# API (FastAPI over SQLite)
cd services/api && uv sync && uv run pytest

# Web (SvelteKit, static)
cd apps/web && pnpm install && pnpm run check && pnpm run build

# The web studio needs the wasm staged first:
bash tools/build-wasm.sh   # builds crates/dsp-wasm -> apps/web/static/dsp/
```

Plugins bundle via `cargo xtask bundle plugin-phaseplan --release` (and
`plugin-morph`) — see [`docs/plugins.md`](docs/plugins.md).

## How we work

- **One concern per commit.** A module, an endpoint group, or a page — small,
  reviewable, and green. Write the test in the same commit.
- **Tests are the contract.** Rust: `cargo clippy --workspace -- -D warnings` and
  `cargo test`. API: `uv run pytest`. Web: `pnpm run check` must be 0 errors / 0
  warnings, and `pnpm run build` must succeed. UI changes come with a screenshot
  (light + dark).
- **DSP is real-time-safe.** No allocation, locking, I/O, or panics in any audio
  callback; smooth every parameter. The DSP lives **only** in `dsp-core` and the
  instrument crates — the wasm and plugin wrappers contain no signal processing, so
  the web and the plugin run identical code.
- **Accessibility is not optional.** Calm UI, legible type, warm near-black, AA+
  contrast, `prefers-reduced-motion`, no alarm iconography. Keyboard-reachable controls.
- **Originality.** The instruments are *inspired by* well-known tools but must contain
  **no** third-party code or assets. Keep required credits intact: sound design by
  **Tev** (Morph), meters by **polarity** (MIT).
- **No secrets, ever.** Nothing internal (hosts, tokens, private paths) goes in the
  repo or an artifact; CI runs a residue scan.

## License & sign-off

By contributing you agree your work is licensed under **AGPL-3.0-or-later** (see
[`LICENSE`](LICENSE)). Please add a `Signed-off-by:` line (DCO) to your commits:

```
git commit -s -m "…"
```

## Review

Changes land by lazy consensus (see GOVERNANCE.md): a focused, tested change with a
clear description that doesn't draw objections in a few days carries. A maintainer
merges and reflects anything roadmap-worthy on `/board`.
