"""board.py — the transparency layer: feedback, feature requests + votes, roadmap.

"Consultative democracy": members propose and upvote; the operator curates the
roadmap. Helpers keep the routes thin (return None -> 404; bad input -> ValueError
-> 400). Identity is the Matrix handle (see auth.py).
"""
from __future__ import annotations

import sqlite3

FR_STATUSES = ("open", "planned", "in-progress", "done", "declined")


def _row(r: sqlite3.Row) -> dict:
    return {k: r[k] for k in r.keys()}


# ── roadmap (operator-curated, public read) ──────────────────────────────────
def list_roadmap(conn: sqlite3.Connection) -> list[dict]:
    rows = conn.execute(
        "SELECT id, title, body, status, eta, sort, updated_at "
        "FROM roadmap_entries ORDER BY sort, id"
    ).fetchall()
    return [_row(r) for r in rows]


# ── feature requests + votes ─────────────────────────────────────────────────
def get_feature_request(conn: sqlite3.Connection, rid: int, viewer: str | None) -> dict | None:
    r = conn.execute(
        "SELECT fr.id, fr.handle, fr.title, fr.body, fr.status, fr.created_at, "
        "(SELECT COUNT(*) FROM feature_votes v WHERE v.request_id=fr.id) AS votes, "
        "EXISTS(SELECT 1 FROM feature_votes v WHERE v.request_id=fr.id AND v.handle=?) AS voted "
        "FROM feature_requests fr WHERE fr.id=?",
        (viewer, rid),
    ).fetchone()
    if not r:
        return None
    d = _row(r)
    d["voted"] = bool(d["voted"])
    return d


def list_feature_requests(
    conn: sqlite3.Connection, viewer: str | None, status: str | None = None
) -> list[dict]:
    sql = (
        "SELECT fr.id, fr.handle, fr.title, fr.body, fr.status, fr.created_at, "
        "(SELECT COUNT(*) FROM feature_votes v WHERE v.request_id=fr.id) AS votes, "
        "EXISTS(SELECT 1 FROM feature_votes v WHERE v.request_id=fr.id AND v.handle=?) AS voted "
        "FROM feature_requests fr"
    )
    args: list = [viewer]
    if status:
        sql += " WHERE fr.status=?"
        args.append(status)
    sql += " ORDER BY votes DESC, fr.created_at DESC"
    rows = conn.execute(sql, args).fetchall()
    out = []
    for r in rows:
        d = _row(r)
        d["voted"] = bool(d["voted"])
        out.append(d)
    return out


def create_feature_request(
    conn: sqlite3.Connection, handle: str, title: str, body: str | None
) -> dict:
    title = (title or "").strip()
    if not title:
        raise ValueError("title required")
    cur = conn.execute(
        "INSERT INTO feature_requests(handle, title, body) VALUES(?,?,?)",
        (handle, title, (body or "").strip() or None),
    )
    rid = cur.lastrowid
    # proposing implies a vote for it
    conn.execute(
        "INSERT OR IGNORE INTO feature_votes(request_id, handle) VALUES(?,?)", (rid, handle)
    )
    conn.commit()
    return get_feature_request(conn, rid, handle)


def vote(conn: sqlite3.Connection, rid: int, handle: str, on: bool) -> dict | None:
    if not conn.execute("SELECT 1 FROM feature_requests WHERE id=?", (rid,)).fetchone():
        return None
    if on:
        conn.execute(
            "INSERT OR IGNORE INTO feature_votes(request_id, handle) VALUES(?,?)", (rid, handle)
        )
    else:
        conn.execute(
            "DELETE FROM feature_votes WHERE request_id=? AND handle=?", (rid, handle)
        )
    conn.commit()
    votes = conn.execute(
        "SELECT COUNT(*) FROM feature_votes WHERE request_id=?", (rid,)
    ).fetchone()[0]
    voted = bool(
        conn.execute(
            "SELECT 1 FROM feature_votes WHERE request_id=? AND handle=?", (rid, handle)
        ).fetchone()
    )
    return {"id": rid, "votes": votes, "voted": voted}


# ── feedback (private; write-only from a member's view) ──────────────────────
def submit_feedback(
    conn: sqlite3.Connection, handle: str | None, message: str, context: str | None
) -> dict:
    message = (message or "").strip()
    if not message:
        raise ValueError("message required")
    cur = conn.execute(
        "INSERT INTO feedback(handle, message, context) VALUES(?,?,?)",
        (handle, message, context),
    )
    conn.commit()
    return {"id": cur.lastrowid, "ok": True}
