# RanchSamples API

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
- `RANCHSAMPLES_MATRIX_HS` — set to enable auth (otherwise open dev mode).
- `RS_ADMIN_HANDLES` — comma-separated admin handles.
