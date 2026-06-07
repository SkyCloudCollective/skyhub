"""notify.py — write + read the notifications inbox.

`add()` is called from the write paths (follow / comment / reaction); it never
notifies you about your own action and does NOT commit (the caller's transaction
does). Reads are a calm poll: recent items + an unread count.
"""
from __future__ import annotations

import json
import sqlite3


def add(
    conn: sqlite3.Connection,
    recipient: str | None,
    kind: str,
    actor: str,
    *,
    subject_type: str | None = None,
    subject_id: int | None = None,
    data: dict | None = None,
) -> None:
    if not recipient or recipient == actor:
        return  # no self-notifications
    conn.execute(
        "INSERT INTO notifications(recipient, kind, actor, subject_type, subject_id, data_json) "
        "VALUES(?,?,?,?,?,?)",
        (recipient, kind, actor, subject_type, subject_id, json.dumps(data) if data else None),
    )


def list_for(conn: sqlite3.Connection, recipient: str, limit: int = 30) -> dict:
    rows = conn.execute(
        "SELECT id, kind, actor, subject_type, subject_id, data_json, read_at, created_at "
        "FROM notifications WHERE recipient=? ORDER BY created_at DESC, id DESC LIMIT ?",
        (recipient, limit),
    ).fetchall()
    items = []
    for r in rows:
        d = {k: r[k] for k in r.keys()}
        d["data"] = json.loads(d.pop("data_json")) if d["data_json"] else None
        d["read"] = d.pop("read_at") is not None
        items.append(d)
    unread = conn.execute(
        "SELECT COUNT(*) FROM notifications WHERE recipient=? AND read_at IS NULL", (recipient,)
    ).fetchone()[0]
    return {"items": items, "unread": unread}


def mark_all_read(conn: sqlite3.Connection, recipient: str) -> dict:
    conn.execute(
        "UPDATE notifications SET read_at=datetime('now') WHERE recipient=? AND read_at IS NULL",
        (recipient,),
    )
    conn.commit()
    return {"unread": 0}
