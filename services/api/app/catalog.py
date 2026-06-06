"""catalog.py — read queries over the sample catalog.

Pure query helpers (no FastAPI here) so they're trivially unit-testable against
an in-memory DB. Write paths (uploads, community) land in P1+.
"""
from __future__ import annotations

import json
import sqlite3

# Splice-style browse vocabulary (controlled).
CATEGORIES = [
    "demos", "beats", "instrumentals", "loops", "one-shots", "melodies",
    "vocals", "sfx", "drumkits", "sound-design", "presets", "midi",
]

_SAMPLE_FIELDS = (
    "id, filename, title, contributor, pack, category, kind, instrument, bpm, "
    "musical_key, duration_ms, brightness, noisiness, percussiveness, loudness, "
    "samplerate, channels, bytes, original_format, created_at"
)


def _row(r: sqlite3.Row) -> dict:
    return {k: r[k] for k in r.keys()}


def get_sample(conn: sqlite3.Connection, sample_id: int) -> dict | None:
    r = conn.execute(f"SELECT {_SAMPLE_FIELDS}, peaks_json FROM samples WHERE id=?", (sample_id,)).fetchone()
    if not r:
        return None
    d = _row(r)
    d["peaks"] = json.loads(d.pop("peaks_json")) if d.get("peaks_json") else []
    d["tags"] = [
        {"tag": t["tag"], "source": t["source"], "added_by": t["added_by"]}
        for t in conn.execute(
            "SELECT tag, source, added_by FROM tags WHERE sample_id=? ORDER BY source, tag", (sample_id,)
        )
    ]
    return d


def peaks(conn: sqlite3.Connection, sample_id: int) -> list | None:
    r = conn.execute("SELECT peaks_json FROM samples WHERE id=?", (sample_id,)).fetchone()
    if not r:
        return None
    return json.loads(r["peaks_json"]) if r["peaks_json"] else []


def facets(conn: sqlite3.Connection) -> dict:
    def distinct(col: str) -> list[str]:
        return [
            r[0]
            for r in conn.execute(
                f"SELECT DISTINCT {col} FROM samples WHERE {col} IS NOT NULL AND {col} <> '' ORDER BY {col}"
            )
        ]

    return {
        "categories": distinct("category"),
        "kinds": distinct("kind"),
        "instruments": distinct("instrument"),
        "contributors": distinct("contributor"),
        "packs": distinct("pack"),
        "tags": [r[0] for r in conn.execute("SELECT DISTINCT tag FROM tags ORDER BY tag")],
    }


def search(
    conn: sqlite3.Connection,
    *,
    q: str | None = None,
    category: str | None = None,
    kind: str | None = None,
    instrument: str | None = None,
    contributor: str | None = None,
    pack: str | None = None,
    musical_key: str | None = None,
    bpm_min: float | None = None,
    bpm_max: float | None = None,
    tag: str | None = None,
    limit: int = 60,
    offset: int = 0,
) -> dict:
    where: list[str] = []
    args: list = []
    if q:
        where.append("(s.title LIKE ? OR s.filename LIKE ?)")
        args += [f"%{q}%", f"%{q}%"]
    for col, val in (
        ("category", category), ("kind", kind), ("instrument", instrument),
        ("contributor", contributor), ("pack", pack), ("musical_key", musical_key),
    ):
        if val:
            where.append(f"s.{col} = ?")
            args.append(val)
    if bpm_min is not None:
        where.append("s.bpm >= ?")
        args.append(bpm_min)
    if bpm_max is not None:
        where.append("s.bpm <= ?")
        args.append(bpm_max)
    join = ""
    if tag:
        join = "JOIN tags t ON t.sample_id = s.id AND t.tag = ?"
        args.insert(0, tag)

    where_sql = ("WHERE " + " AND ".join(where)) if where else ""
    total = conn.execute(
        f"SELECT COUNT(DISTINCT s.id) FROM samples s {join} {where_sql}", args
    ).fetchone()[0]

    limit = max(1, min(int(limit), 200))
    offset = max(0, int(offset))
    rows = conn.execute(
        f"SELECT DISTINCT {_sample_select('s')} FROM samples s {join} {where_sql} "
        f"ORDER BY s.created_at DESC, s.id DESC LIMIT ? OFFSET ?",
        args + [limit, offset],
    ).fetchall()
    return {"total": total, "limit": limit, "offset": offset, "hits": [_row(r) for r in rows]}


def _sample_select(alias: str) -> str:
    return ", ".join(f"{alias}.{f.strip()}" for f in _SAMPLE_FIELDS.split(","))


def resolve_file(conn: sqlite3.Connection, sample_id: int, prefer_preview: bool = True):
    """Return (rel_path, filename) for serving, preferring a browser-playable
    preview if one was transcoded. None if the sample doesn't exist."""
    r = conn.execute(
        "SELECT rel_path, filename, preview_rel FROM samples WHERE id=?", (sample_id,)
    ).fetchone()
    if not r:
        return None
    if prefer_preview and r["preview_rel"]:
        return (r["preview_rel"], r["filename"], True)
    return (r["rel_path"], r["filename"], False)
