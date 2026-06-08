# Self-hosting SkyHub

SkyHub is built to be run by anyone, on one small box. The whole platform
is **one FastAPI process over a single SQLite file**, plus a **prebuilt static
web app** (no Node runtime in production). The Python worker is only needed to
analyse audio when you add it — not to keep the site up.

There is no SaaS dependency, no database server, and no message queue.

---

## TL;DR (single process, single origin)

The API can serve the built web app itself, so the simplest deploy is one
command. The app then talks to the API same-origin (no CORS, tight CSP).

```sh
just setup                       # toolchains (rust, node, python) — once
just build-web                   # → apps/web/build (static SPA)
just build-wasm                  # → the studio's DSP (apps/web/static/dsp/*.wasm)

# Serve API + SPA from one process on :8000
RANCHSAMPLES_DB=./data/catalog.db \
RANCHSAMPLES_ROOT=./samples/synthetic \
RANCHSAMPLES_WEB=./apps/web/build \
  uv --project services/api run uvicorn app.main:app --host 0.0.0.0 --port 8000
```

Open `http://<host>:8000/`. That's a working instance.

---

## Production (Caddy, TLS, single origin)

For a public instance, put Caddy in front: it terminates TLS, serves the static
SPA, and reverse-proxies `/v1/*` to uvicorn on the loopback. A ready example
lives at [`ops/Caddyfile.example`](ops/Caddyfile.example) — copy it, set your
domain, point `root` at the built `apps/web/build`, and run uvicorn bound to
`127.0.0.1:8000` (drop `RANCHSAMPLES_WEB`, since Caddy serves the files).

The example ships a strict CSP (`connect-src 'self'`, `object-src 'none'`,
`frame-ancestors 'none'`) and the COOP/COEP headers the studio uses.

Run uvicorn under a process supervisor (a `systemd` unit or your container
runtime). Behind a supervisor, `--workers 2` is plenty for a community instance;
SQLite is opened in WAL mode.

---

## The sample library

The catalog is a folder of audio whose **path encodes provenance**, plus a
SQLite row per file (BPM/key/timbre, tags, waveform peaks):

```
packs/<slug>/...            → pack = <slug>
contributors/<handle>/...   → contributor = <handle>
```

Two ways to fill it:

- **Your own folder** — drop audio into `RANCHSAMPLES_ROOT` following the layout
  above, then `cd services/worker && uv run python ingest.py` to analyse + index.
- **Import an existing catalog** — `services/worker/import_v1.py` copies a prior
  SkyHub catalog (curated rows + human/system tags) and recomputes timbre.
  See [`services/worker/README.md`](services/worker/README.md).

The DB (`data/*.db`) and the audio (`samples/`) never live in git — they are your
instance's data.

---

## Configuration

Copy [`.env.example`](.env.example) and set what you need (never commit a real
env). The essentials:

| Variable | What it does |
|---|---|
| `RANCHSAMPLES_DB` | path to the SQLite catalog (default `./data/catalog.db`) |
| `RANCHSAMPLES_ROOT` | folder that holds the audio (default `./samples/synthetic`) |
| `RANCHSAMPLES_WEB` | built SPA dir to serve single-origin (omit when Caddy serves it) |
| `RS_ADMIN_HANDLES` | comma-separated admin handles |
| `RS_BASIC_AUTH` | `user:password` — locks every route except `/healthz` (a private preview) |
| `RS_OPEN_UGC` | `0`/`1` — public open-collaboration projects, **off by default** |

`RS_BASIC_AUTH` is the front-door lock for a **private preview**: share the URL +
credentials with your circle, keep everything gated. `RS_OPEN_UGC` stays off
until you have done your own legal due diligence on user-contributed content.

---

## Containers

A `Containerfile` builds SkyHub from source in three stages (Rust → the studio
wasm, Node → the static SPA, Python+uv → the runtime) into one image that runs
the API serving the SPA single-origin. Mount your library + DB at `/data`:

```sh
podman build -t skyhub:latest -f Containerfile .
podman run --rm -p 8000:8000 \
  -v ./data/catalog.db:/data/catalog.db:Z \
  -v ./samples/library:/data/samples:ro,Z \
  skyhub:latest
```

Open `http://<host>:8000/`. The image is also published to GHCR on each release
(`ghcr.io/<owner>/skyhub:<tag>`), so you can skip the build:
`podman run -p 8000:8000 -v ./data:/data:Z ghcr.io/<owner>/skyhub:latest`.

A Compose file (app + a reverse proxy) is the next step.
