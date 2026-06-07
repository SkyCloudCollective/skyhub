# Containerfile — SkyHub: the API serving the static web app over a SQLite catalog.
# One image, one process (uvicorn). Built from source in three stages:
#   wasm (Rust → the studio's DSP)  →  web (Node → static SPA)  →  runtime (Python + uv).
# No SaaS, no Node runtime in production. Mount your library + DB at /data.
#
#   podman build -t skyhub:dev -f Containerfile .
#   podman run --rm -p 8000:8000 -v ./data:/data:Z skyhub:dev

# ---- stage 1: the studio DSP, compiled to WebAssembly -----------------------
FROM docker.io/library/rust:1-bookworm AS wasm
WORKDIR /src
RUN rustup target add wasm32-unknown-unknown
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build -p dsp-wasm --release --target wasm32-unknown-unknown

# ---- stage 2: the static web app (SvelteKit, adapter-static) ----------------
FROM docker.io/library/node:22-bookworm AS web
RUN corepack enable
WORKDIR /src
COPY pnpm-workspace.yaml package.json pnpm-lock.yaml ./
COPY packages ./packages
COPY apps ./apps
RUN pnpm install --frozen-lockfile
# stage the compiled wasm where the studio worklet loads it, then build
COPY --from=wasm /src/target/wasm32-unknown-unknown/release/dsp_wasm.wasm \
     apps/web/static/dsp/dsp_wasm.wasm
RUN cd apps/web && pnpm build

# ---- stage 3: runtime — the FastAPI service serving the SPA single-origin ----
FROM docker.io/library/python:3.12-slim-bookworm AS runtime
COPY --from=ghcr.io/astral-sh/uv:latest /uv /usr/local/bin/uv
WORKDIR /app
COPY services/api ./services/api
COPY db ./db
RUN cd services/api && uv sync --frozen --no-dev
COPY --from=web /src/apps/web/build ./apps/web/build

ENV RANCHSAMPLES_WEB=/app/apps/web/build \
    RANCHSAMPLES_DB=/data/catalog.db \
    RANCHSAMPLES_ROOT=/data/samples \
    RANCHSAMPLES_PROJECTS=/data/projects
VOLUME /data
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=4s --start-period=20s \
  CMD python3 -c "import urllib.request,sys; sys.exit(0 if urllib.request.urlopen('http://127.0.0.1:8000/healthz',timeout=3).status==200 else 1)" || exit 1

WORKDIR /app/services/api
CMD ["uv", "run", "--no-dev", "uvicorn", "app.main:app", "--host", "0.0.0.0", "--port", "8000"]
