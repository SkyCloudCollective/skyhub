"""social.py — comments + emoji reactions on samples.

Thin helpers (route stays thin): missing sample -> None (404); empty body /
bad emoji -> ValueError (400); deleting someone else's comment -> PermissionError
(403). Identity is the Matrix handle (auth.py). Notifications are wired in C3.
"""
from __future__ import annotations

import sqlite3

from . import notify

MAX_EMOJI_LEN = 8


def _sample(conn: sqlite3.Connection, sample_id: int) -> sqlite3.Row | None:
    return conn.execute(
        "SELECT id, title, contributor FROM samples WHERE id=?", (sample_id,)
    ).fetchone()


# ── comments ─────────────────────────────────────────────────────────────────
def list_comments(conn: sqlite3.Connection, sample_id: int) -> list[dict]:
    rows = conn.execute(
        "SELECT id, sample_id, parent_id, handle, body, edited_at, created_at "
        "FROM comments WHERE sample_id=? ORDER BY created_at, id",
        (sample_id,),
    ).fetchall()
    return [{k: r[k] for k in r.keys()} for r in rows]


def add_comment(
    conn: sqlite3.Connection,
    sample_id: int,
    handle: str,
    body: str,
    parent_id: int | None = None,
) -> dict | None:
    s = _sample(conn, sample_id)
    if s is None:
        return None
    body = (body or "").strip()
    if not body:
        raise ValueError("empty comment")
    if parent_id is not None and not conn.execute(
        "SELECT 1 FROM comments WHERE id=? AND sample_id=?", (parent_id, sample_id)
    ).fetchone():
        raise ValueError("bad parent")
    cur = conn.execute(
        "INSERT INTO comments(sample_id, parent_id, handle, body) VALUES(?,?,?,?)",
        (sample_id, parent_id, handle, body),
    )
    notify.add(
        conn, s["contributor"], "comment", handle,
        subject_type="sample", subject_id=sample_id, data={"title": s["title"]},
    )
    conn.commit()
    r = conn.execute("SELECT * FROM comments WHERE id=?", (cur.lastrowid,)).fetchone()
    return {k: r[k] for k in r.keys()}


def delete_comment(conn: sqlite3.Connection, comment_id: int, handle: str) -> bool | None:
    r = conn.execute("SELECT handle FROM comments WHERE id=?", (comment_id,)).fetchone()
    if not r:
        return None
    if r["handle"] != handle:
        raise PermissionError("not the author")
    conn.execute("DELETE FROM comments WHERE id=?", (comment_id,))
    conn.commit()
    return True


# ── reactions (emoji) ────────────────────────────────────────────────────────
def reactions(conn: sqlite3.Connection, sample_id: int, viewer: str | None) -> dict:
    rows = conn.execute(
        "SELECT emoji, COUNT(*) AS n FROM reactions WHERE sample_id=? GROUP BY emoji ORDER BY n DESC",
        (sample_id,),
    ).fetchall()
    counts = {r["emoji"]: r["n"] for r in rows}
    mine = [
        r["emoji"]
        for r in conn.execute(
            "SELECT emoji FROM reactions WHERE sample_id=? AND handle=?", (sample_id, viewer)
        )
    ]
    return {"counts": counts, "mine": mine}


def toggle_reaction(
    conn: sqlite3.Connection, sample_id: int, handle: str, emoji: str, on: bool
) -> dict | None:
    s = _sample(conn, sample_id)
    if s is None:
        return None
    emoji = (emoji or "").strip()
    if not emoji or len(emoji) > MAX_EMOJI_LEN:
        raise ValueError("bad emoji")
    if on:
        conn.execute(
            "INSERT OR IGNORE INTO reactions(sample_id, handle, emoji) VALUES(?,?,?)",
            (sample_id, handle, emoji),
        )
        notify.add(
            conn, s["contributor"], "reaction", handle,
            subject_type="sample", subject_id=sample_id, data={"title": s["title"], "emoji": emoji},
        )
    else:
        conn.execute(
            "DELETE FROM reactions WHERE sample_id=? AND handle=? AND emoji=?",
            (sample_id, handle, emoji),
        )
    conn.commit()
    return reactions(conn, sample_id, handle)
