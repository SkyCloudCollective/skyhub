"""db.py — SQLite access for the RanchSamples catalog.

The schema is NOT defined here: it lives in db/schema.sql at the repo root (the
canonical source of truth, shared with the worker and the migration runner).
This module opens connections (WAL, foreign keys), applies the schema on a
fresh DB, and runs the forward-only migrations in db/migrations/.
"""
from __future__ import annotations

import os
import pathlib
import sqlite3

# repo root = services/api/app/db.py -> up 3
REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
SCHEMA_PATH = pathlib.Path(
    os.environ.get("RANCHSAMPLES_SCHEMA", REPO_ROOT / "db" / "schema.sql")
)
MIGRATIONS_DIR = pathlib.Path(
    os.environ.get("RANCHSAMPLES_MIGRATIONS", REPO_ROOT / "db" / "migrations")
)
DB_PATH = pathlib.Path(
    os.environ.get("RANCHSAMPLES_DB", REPO_ROOT / "data" / "catalog.db")
)
# Where audio files actually live (synthetic locally; the studio volume in prod).
SAMPLES_ROOT = pathlib.Path(
    os.environ.get("RANCHSAMPLES_ROOT", REPO_ROOT / "samples" / "synthetic")
)
# Transcoded browser-playable previews (for originals that aren't, e.g. aiff).
PREVIEWS_ROOT = pathlib.Path(
    os.environ.get("RANCHSAMPLES_PREVIEWS", REPO_ROOT / "_meta" / "previews")
)
# Collaborative project files (local stub backend; a Nextcloud group folder in prod).
PROJECTS_ROOT = pathlib.Path(
    os.environ.get("RANCHSAMPLES_PROJECTS", REPO_ROOT / "data" / "projects")
)


def connect(db_path: str | os.PathLike | None = None, read_only: bool = False) -> sqlite3.Connection:
    path = pathlib.Path(db_path) if db_path else DB_PATH
    path.parent.mkdir(parents=True, exist_ok=True)
    if read_only and path.exists():
        conn = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    else:
        conn = sqlite3.connect(path)
        conn.execute("PRAGMA journal_mode = WAL")
        conn.execute("PRAGMA synchronous = NORMAL")
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys = ON")
    conn.execute("PRAGMA busy_timeout = 5000")
    return conn


def _apply_migrations(conn: sqlite3.Connection) -> None:
    """Run any *.sql in db/migrations/ not yet recorded, in filename order."""
    if not MIGRATIONS_DIR.exists():
        return
    done = {r["version"] for r in conn.execute("SELECT version FROM schema_migrations")}
    for sql_file in sorted(MIGRATIONS_DIR.glob("*.sql")):
        version = sql_file.stem
        if version in done:
            continue
        conn.executescript(sql_file.read_text(encoding="utf-8"))
        conn.execute("INSERT OR IGNORE INTO schema_migrations(version) VALUES (?)", (version,))
        conn.commit()


def init_db(db_path: str | os.PathLike | None = None) -> None:
    """Apply the canonical schema (idempotent) + pending migrations."""
    conn = connect(db_path)
    try:
        conn.executescript(SCHEMA_PATH.read_text(encoding="utf-8"))
        _apply_migrations(conn)
        conn.commit()
    finally:
        conn.close()
