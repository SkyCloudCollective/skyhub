"""main.py — the single RanchSamples API (FastAPI).

Replaces the v1 dual server (stdlib serve.py + FastAPI app.py kept in lockstep).
One implementation, OpenAPI docs for free, the same /v1 surface. Read endpoints
land in P0; write paths (uploads, community) follow in P1+.
"""
from __future__ import annotations

import mimetypes
import pathlib
import sqlite3
from contextlib import asynccontextmanager
from typing import Optional

from fastapi import Depends, FastAPI, HTTPException, Query
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse

from . import __version__, auth, catalog
from .db import PREVIEWS_ROOT, SAMPLES_ROOT, connect, init_db


@asynccontextmanager
async def lifespan(app: FastAPI):
    init_db()  # apply canonical schema + migrations on boot
    yield


app = FastAPI(
    title="RanchSamples API",
    version=__version__,
    description="Community-owned audio-samples platform.",
    lifespan=lifespan,
)

# The desktop shell and the static web app are different origins; allow GETs.
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["GET", "POST", "PATCH", "DELETE", "OPTIONS"],
    allow_headers=["*"],
)


def db() -> sqlite3.Connection:
    conn = connect(read_only=True)
    try:
        yield conn
    finally:
        conn.close()


# ── discovery ────────────────────────────────────────────────────────────────
@app.get("/v1/health")
def health():
    return {"status": "ok", "service": "ranchsamples-api", "version": __version__}


@app.get("/v1/me")
def me():
    return {"auth_enabled": auth.auth_enabled(), "handle": auth.current_handle()}


# ── catalog ──────────────────────────────────────────────────────────────────
@app.get("/v1/facets")
def get_facets(conn: sqlite3.Connection = Depends(db)):
    return catalog.facets(conn)


@app.get("/v1/search")
def get_search(
    q: Optional[str] = None,
    category: Optional[str] = None,
    kind: Optional[str] = None,
    instrument: Optional[str] = None,
    contributor: Optional[str] = None,
    pack: Optional[str] = None,
    key: Optional[str] = Query(default=None, alias="key"),
    bpm_min: Optional[float] = None,
    bpm_max: Optional[float] = None,
    tag: Optional[str] = None,
    limit: int = 60,
    offset: int = 0,
    conn: sqlite3.Connection = Depends(db),
):
    return catalog.search(
        conn, q=q, category=category, kind=kind, instrument=instrument,
        contributor=contributor, pack=pack, musical_key=key,
        bpm_min=bpm_min, bpm_max=bpm_max, tag=tag, limit=limit, offset=offset,
    )


@app.get("/v1/sample/{sample_id}")
def get_sample(sample_id: int, conn: sqlite3.Connection = Depends(db)):
    s = catalog.get_sample(conn, sample_id)
    if not s:
        raise HTTPException(status_code=404, detail="sample not found")
    return s


@app.get("/v1/peaks/{sample_id}")
def get_peaks(sample_id: int, conn: sqlite3.Connection = Depends(db)):
    p = catalog.peaks(conn, sample_id)
    if p is None:
        raise HTTPException(status_code=404, detail="sample not found")
    return p


def _safe_abs(rel: str, is_preview: bool) -> pathlib.Path:
    """Resolve a catalog rel_path under its root, guarding against traversal."""
    base = (PREVIEWS_ROOT if is_preview else SAMPLES_ROOT).resolve()
    target = (base / rel).resolve()
    if base != target and base not in target.parents:
        raise HTTPException(status_code=404, detail="not found")
    if not target.is_file():
        raise HTTPException(status_code=404, detail="file missing")
    return target


@app.get("/v1/preview/{sample_id}")
def preview(sample_id: int, conn: sqlite3.Connection = Depends(db)):
    # Browser-playable preview (Range supported by FileResponse → seekable <audio>).
    resolved = catalog.resolve_file(conn, sample_id, prefer_preview=True)
    if not resolved:
        raise HTTPException(status_code=404, detail="sample not found")
    rel, filename, is_preview = resolved
    path = _safe_abs(rel, is_preview)
    media = mimetypes.guess_type(filename)[0] or "application/octet-stream"
    return FileResponse(path, media_type=media)


@app.get("/v1/download/{sample_id}")
def download(sample_id: int, conn: sqlite3.Connection = Depends(db)):
    # The original, full-quality file as an attachment (real bytes for the DAW).
    resolved = catalog.resolve_file(conn, sample_id, prefer_preview=False)
    if not resolved:
        raise HTTPException(status_code=404, detail="sample not found")
    rel, filename, _ = resolved
    path = _safe_abs(rel, False)
    return FileResponse(
        path,
        media_type="application/octet-stream",
        filename=filename,
        headers={"Content-Disposition": f'attachment; filename="{filename}"'},
    )
