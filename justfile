# RanchSamples — one task runner fronting every toolchain.
# `just` (https://github.com/casey/just). Run `just` to list recipes.

set shell := ["bash", "-uc"]

# default: list recipes
default:
    @just --list

# ── setup ───────────────────────────────────────────────────────────────────
# Sync all toolchains. Rust via rustup, JS via pnpm, Python via uv.
setup: setup-rust setup-web setup-api

setup-rust:
    rustup target add wasm32-unknown-unknown
    cargo --version

setup-web:
    cd apps/web && pnpm install

setup-api:
    cd services/api && uv sync

# ── dev ─────────────────────────────────────────────────────────────────────
# API (FastAPI, :8000) and web (SvelteKit, :5173) in watch mode. Run in 2 shells.
dev-api:
    cd services/api && uv run uvicorn app.main:app --reload --port 8000

dev-web:
    cd apps/web && pnpm dev

# ── test ────────────────────────────────────────────────────────────────────
test: test-rust test-api

test-rust:
    cargo test --workspace

test-api:
    cd services/api && uv run pytest -q

# ── build ───────────────────────────────────────────────────────────────────
build-web:
    cd apps/web && pnpm build

build-wasm:
    cargo build -p dsp-wasm --release --target wasm32-unknown-unknown

# native plugins (host arch). On a Mac this yields macOS bundles; on Linux, .so/.clap.
build-plugins:
    cargo build -p plugin-botanica -p plugin-phaseplan --release

# bundle Linux plugins into dist/linux-x86_64/ (CLAP + VST3)
bundle-linux:
    bash scripts/build-plugins-linux.sh --release

# bundle Windows plugins from Linux via cargo-xwin into dist/windows-x86_64/
# Prerequisites: cargo install cargo-xwin && rustup target add x86_64-pc-windows-msvc
bundle-windows:
    bash scripts/build-plugins-windows.sh

# build all reachable platforms + manifest (set RS_MAC_SSH_HOST for macOS)
bundle-all:
    bash scripts/build-all.sh

# remote macOS build (requires RS_MAC_SSH_HOST env var)
bundle-mac:
    bash scripts/build-mac-remote.sh

# ── quality gates ─────────────────────────────────────────────────────────────
fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace -- -D warnings

# residue scan on built artifacts before any publish (see ci/audit-artifact.py)
audit dir="dist":
    python3 ci/audit-artifact.py {{dir}}
