"""db.py — worker-side catalog writes (same DB + schema as the API).

The worker analyses files and upserts rows; the API only reads. Both point at
the same SQLite file (RANCHSAMPLES_DB) and the same canonical schema
(db/schema.sql), so there is one source of truth, written by one service.
"""
from __future__ import annotations

import os
import pathlib
import sqlite3

REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]
SCHEMA_PATH = pathlib.Path(os.environ.get("RANCHSAMPLES_SCHEMA", REPO_ROOT / "db" / "schema.sql"))
MIGRATIONS_DIR = pathlib.Path(
    os.environ.get("RANCHSAMPLES_MIGRATIONS", REPO_ROOT / "db" / "migrations")
)
DB_PATH = pathlib.Path(os.environ.get("RANCHSAMPLES_DB", REPO_ROOT / "data" / "catalog.db"))
SAMPLES_ROOT = pathlib.Path(
    os.environ.get("RANCHSAMPLES_ROOT", REPO_ROOT / "samples" / "synthetic")
)

_UPSERT_COLS = [
    "rel_path", "filename", "title", "contributor", "pack", "category", "kind",
    "instrument", "bpm", "musical_key", "bpm_source", "duration_ms",
    "brightness", "noisiness", "percussiveness", "loudness",
    "samplerate", "channels", "bitdepth", "bytes", "sha256", "license",
    "original_format", "preview_rel", "peaks_json", "origin",
]


def classify_origin(rel_path: str) -> str:
    """A member's own work ('original') vs a collected commercial pack ('licensed').

    Heuristic, identical to migration 0003's SQL backfill: anything under a
    '.../Packs/...' path is a third-party pack the member merely collected, so it
    is kept private and never served in the shared catalog. New ingests classify
    themselves here so the gate stays correct without a manual backfill.
    """
    return "licensed" if "/packs/" in rel_path.lower() else "original"


def connect(db_path: str | os.PathLike | None = None) -> sqlite3.Connection:
    path = pathlib.Path(db_path) if db_path else DB_PATH
    path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(path)
    conn.execute("PRAGMA journal_mode = WAL")
    conn.execute("PRAGMA synchronous = NORMAL")
    conn.execute("PRAGMA foreign_keys = ON")
    conn.row_factory = sqlite3.Row
    return conn


def init_db(conn: sqlite3.Connection) -> None:
    """Canonical schema + forward-only migrations (same as the API's init_db, so a
    worker-created DB and an API-created DB are identical — incl. samples.origin)."""
    conn.executescript(SCHEMA_PATH.read_text(encoding="utf-8"))
    if MIGRATIONS_DIR.exists():
        done = {r[0] for r in conn.execute("SELECT version FROM schema_migrations")}
        for sql_file in sorted(MIGRATIONS_DIR.glob("*.sql")):
            if sql_file.stem in done:
                continue
            conn.executescript(sql_file.read_text(encoding="utf-8"))
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?)", (sql_file.stem,)
            )
    conn.commit()


def upsert_sample(conn: sqlite3.Connection, rec: dict, tags: list[tuple[str, str]]) -> int:
    """Insert/update by rel_path. `tags` = list of (tag, source)."""
    # Classify a member's own work vs a collected commercial pack, unless the
    # caller already declared it (e.g. an upload's explicit origin declaration).
    rec.setdefault("origin", classify_origin(rec.get("rel_path", "")))
    values = [rec.get(c) for c in _UPSERT_COLS]
    placeholders = ",".join("?" for _ in _UPSERT_COLS)
    update = ",".join(f"{c}=excluded.{c}" for c in _UPSERT_COLS if c != "rel_path")
    conn.execute(
        f"INSERT INTO samples ({','.join(_UPSERT_COLS)}) VALUES ({placeholders}) "
        f"ON CONFLICT(rel_path) DO UPDATE SET {update}, indexed_at=datetime('now')",
        values,
    )
    sid = conn.execute("SELECT id FROM samples WHERE rel_path=?", (rec["rel_path"],)).fetchone()[0]
    # refresh auto/system tags for this sample (leave user tags intact)
    conn.execute("DELETE FROM tags WHERE sample_id=? AND source IN ('auto','system')", (sid,))
    for tag, source in tags:
        conn.execute(
            "INSERT OR IGNORE INTO tags(sample_id,tag,source) VALUES(?,?,?)", (sid, tag, source)
        )
    conn.commit()
    return sid
