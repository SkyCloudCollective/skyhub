# SkyHub API

A single FastAPI service over the SQLite catalog. Replaces the v1 dual server
(stdlib + FastAPI kept in lockstep) — one implementation, OpenAPI docs for free.

## Run

```sh
uv sync                 # install into a local .venv (Python 3.12)
uv run uvicorn app.main:app --reload --port 8000
# docs at http://127.0.0.1:8000/docs
```

## Test

```sh
uv run pytest
```

## Layout

- `app/db.py` — connections (WAL, FK) + applies `db/schema.sql` and `db/migrations/`.
- `app/catalog.py` — read queries (search, facets, sample, peaks). Pure, unit-testable.
- `app/auth.py` — Matrix-handle identity + admin set. Dormant until a homeserver is set.
- `app/main.py` — the `/v1` routes.

## Config (env)

- `RANCHSAMPLES_DB` — catalog path (default `data/catalog.db` at repo root).
- `RANCHSAMPLES_ROOT` — where audio files live.
- `RANCHSAMPLES_WEB` — built SvelteKit dir; if set, the API also serves the SPA (single origin).
- `RANCHSAMPLES_MATRIX_HS` — set to enable auth (otherwise open dev mode).
- `RS_ADMIN_HANDLES` — comma-separated admin handles.
- `RS_OPEN_UGC` — truthy to allow `open` (public) project collaboration (off until the legal pass).
- `RS_BASIC_AUTH` — `"user:password"`. When set, a coarse HTTP Basic gate protects
  **every** route except `/healthz` (401 + `WWW-Authenticate: Basic realm="SkyHub"`,
  constant-time compare). Unset = no gate (tests/local untouched). It's the only consumer of
  the `Authorization` header; per-member identity rides on the `X-RS-Handle` header, which the
  web client sends and which takes priority in `acting()` — so the gate and identity never
  collide. Runtime-only; never commit a real value.

## RanchTube (`/v1/tube`)

Community video, embeds only (phase 1). `POST /v1/tube` accepts `{url,title,description?,category?}`;
the server validates the provider against an allowlist (YouTube/Vimeo/PeerTube, https only),
extracts a bare `video_ref`, and stores only `(provider, video_ref)` — never the raw URL. The
embed `src` is rebuilt from that pair (anti iframe-injection / XSS). `GET /v1/tube` lists recent
first with `handle`/`category` filters; `GET /v1/tube/{id}` fetches one. Reactions/comments reuse
the generic `social` layer (`subject_type='video'`).
