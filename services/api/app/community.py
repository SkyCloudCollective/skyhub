"""community.py — profiles, collections (crates/favorites/smart), membership.

Write + read helpers for the community layer. Ownership and visibility are
enforced here so the routes stay thin:
  * not found        -> returns None / [] (route raises 404)
  * not the owner     -> raises PermissionError (route raises 403)

Identity is the Matrix handle (see auth.py). Favourites are NOT a separate
table: each user has one system collection (kind='favorites'), so "favourite"
and "organise into crates" share one model.
"""
from __future__ import annotations

import json
import secrets
import sqlite3

from . import notify

VISIBILITIES = ("private", "unlisted", "public")


# ── profiles ─────────────────────────────────────────────────────────────────
def get_profile(conn: sqlite3.Connection, handle: str, viewer: str | None = None) -> dict:
    r = conn.execute("SELECT * FROM profiles WHERE handle=?", (handle,)).fetchone()
    sample_count = conn.execute(
        "SELECT COUNT(*) FROM samples WHERE contributor=?", (handle,)
    ).fetchone()[0]
    base = {
        "handle": handle,
        "display_name": None,
        "bio": None,
        "avatar_path": None,
        "links": [],
    }
    if r:
        base.update(
            {
                "display_name": r["display_name"],
                "bio": r["bio"],
                "avatar_path": r["avatar_path"],
                "links": json.loads(r["links_json"]) if r["links_json"] else [],
            }
        )
    base["sample_count"] = sample_count
    base.update(follow_status(conn, handle, viewer))
    return base


# ── follows (the social graph) ───────────────────────────────────────────────
def follow_status(conn: sqlite3.Connection, handle: str, viewer: str | None) -> dict:
    followers = conn.execute(
        "SELECT COUNT(*) FROM follows WHERE followed=?", (handle,)
    ).fetchone()[0]
    following = conn.execute(
        "SELECT COUNT(*) FROM follows WHERE follower=?", (handle,)
    ).fetchone()[0]
    you_follow = bool(
        viewer
        and conn.execute(
            "SELECT 1 FROM follows WHERE follower=? AND followed=?", (viewer, handle)
        ).fetchone()
    )
    return {"followers": followers, "following": following, "you_follow": you_follow}


def set_follow(conn: sqlite3.Connection, follower: str, followed: str, on: bool) -> dict:
    if follower == followed:
        raise ValueError("cannot follow yourself")
    if on:
        conn.execute(
            "INSERT OR IGNORE INTO follows(follower, followed) VALUES(?,?)", (follower, followed)
        )
        notify.add(conn, followed, "follow", follower)
    else:
        conn.execute("DELETE FROM follows WHERE follower=? AND followed=?", (follower, followed))
    conn.commit()
    return follow_status(conn, followed, follower)


def following_of(conn: sqlite3.Connection, handle: str) -> list[str]:
    return [
        r[0]
        for r in conn.execute(
            "SELECT followed FROM follows WHERE follower=? ORDER BY created_at DESC", (handle,)
        )
    ]


def feed(conn: sqlite3.Connection, viewer: str, limit: int = 30) -> list[dict]:
    """Recent samples contributed by the people `viewer` follows."""
    rows = conn.execute(
        "SELECT s.id, s.filename, s.title, s.contributor, s.pack, s.category, s.kind, "
        "s.instrument, s.bpm, s.musical_key, s.duration_ms, s.created_at "
        "FROM samples s "
        "WHERE s.contributor IN (SELECT followed FROM follows WHERE follower=?) "
        "ORDER BY s.created_at DESC, s.id DESC LIMIT ?",
        (viewer, limit),
    ).fetchall()
    return [{k: r[k] for k in r.keys()} for r in rows]


def upsert_profile(
    conn: sqlite3.Connection,
    handle: str,
    *,
    display_name: str | None = None,
    bio: str | None = None,
    links: list | None = None,
) -> dict:
    links_json = json.dumps(links) if links is not None else None
    conn.execute(
        "INSERT INTO profiles(handle, display_name, bio, links_json) VALUES(?,?,?,?) "
        "ON CONFLICT(handle) DO UPDATE SET "
        "display_name=COALESCE(excluded.display_name, profiles.display_name), "
        "bio=COALESCE(excluded.bio, profiles.bio), "
        "links_json=COALESCE(excluded.links_json, profiles.links_json), "
        "updated_at=datetime('now')",
        (handle, display_name, bio, links_json),
    )
    conn.commit()
    return get_profile(conn, handle)


# ── collections ──────────────────────────────────────────────────────────────
def _row(r: sqlite3.Row) -> dict:
    return {k: r[k] for k in r.keys()}


def ensure_favorites(conn: sqlite3.Connection, owner: str) -> int:
    r = conn.execute(
        "SELECT id FROM collections WHERE owner=? AND kind='favorites'", (owner,)
    ).fetchone()
    if r:
        return r["id"]
    cur = conn.execute(
        "INSERT INTO collections(owner,name,kind,visibility) VALUES(?,?,?,?)",
        (owner, "Favorites", "favorites", "private"),
    )
    conn.commit()
    return cur.lastrowid


def list_collections(conn: sqlite3.Connection, owner: str, viewer: str | None) -> list[dict]:
    rows = conn.execute(
        "SELECT * FROM collections WHERE owner=? ORDER BY kind!='favorites', position, id", (owner,)
    ).fetchall()
    out = []
    for r in rows:
        d = _row(r)
        if viewer != owner and d["visibility"] != "public":
            continue
        d["count"] = conn.execute(
            "SELECT COUNT(*) FROM collection_items WHERE collection_id=?", (d["id"],)
        ).fetchone()[0]
        out.append(d)
    return out


def create_collection(
    conn: sqlite3.Connection, owner: str, name: str, kind: str = "crate",
    visibility: str = "private",
) -> dict:
    if visibility not in VISIBILITIES:
        raise ValueError("bad visibility")
    if kind == "favorites":  # only the system path creates these
        kind = "crate"
    cur = conn.execute(
        "INSERT INTO collections(owner,name,kind,visibility) VALUES(?,?,?,?)",
        (owner, name.strip() or "Untitled", kind, visibility),
    )
    conn.commit()
    return get_collection(conn, cur.lastrowid, owner)["collection"]


def _visible(coll: dict, viewer: str | None, token: str | None) -> bool:
    if viewer == coll["owner"]:
        return True
    if coll["visibility"] == "public":
        return True
    if coll["visibility"] == "unlisted" and token and token == coll["share_token"]:
        return True
    return False


def get_collection(
    conn: sqlite3.Connection, cid: int, viewer: str | None, token: str | None = None
) -> dict | None:
    r = conn.execute("SELECT * FROM collections WHERE id=?", (cid,)).fetchone()
    if not r:
        return None
    coll = _row(r)
    if not _visible(coll, viewer, token):
        raise PermissionError("not visible")
    items = conn.execute(
        "SELECT s.id, s.filename, s.title, s.contributor, s.category, s.kind, "
        "s.instrument, s.bpm, s.musical_key, s.duration_ms, ci.position "
        "FROM collection_items ci JOIN samples s ON s.id=ci.sample_id "
        "WHERE ci.collection_id=? ORDER BY ci.position, ci.added_at",
        (cid,),
    ).fetchall()
    return {"collection": coll, "items": [_row(i) for i in items]}


def _own_or_403(conn: sqlite3.Connection, cid: int, owner: str) -> dict:
    r = conn.execute("SELECT * FROM collections WHERE id=?", (cid,)).fetchone()
    if not r:
        return None
    if r["owner"] != owner:
        raise PermissionError("not owner")
    return _row(r)


def update_collection(conn: sqlite3.Connection, cid: int, owner: str, fields: dict) -> dict | None:
    coll = _own_or_403(conn, cid, owner)
    if not coll:
        return None
    sets, args = [], []
    if "name" in fields and fields["name"] is not None:
        sets.append("name=?")
        args.append(str(fields["name"]).strip() or "Untitled")
    if "visibility" in fields and fields["visibility"] in VISIBILITIES:
        sets.append("visibility=?")
        args.append(fields["visibility"])
        # mint a share token when first making it unlisted
        if fields["visibility"] == "unlisted" and not coll.get("share_token"):
            sets.append("share_token=?")
            args.append(secrets.token_urlsafe(12))
    if "position" in fields and fields["position"] is not None:
        sets.append("position=?")
        args.append(int(fields["position"]))
    if sets:
        sets.append("updated_at=datetime('now')")
        conn.execute(f"UPDATE collections SET {','.join(sets)} WHERE id=?", args + [cid])
        conn.commit()
    return get_collection(conn, cid, owner)["collection"]


def delete_collection(conn: sqlite3.Connection, cid: int, owner: str) -> bool:
    coll = _own_or_403(conn, cid, owner)
    if not coll:
        return False
    if coll["kind"] == "favorites":
        raise PermissionError("cannot delete the favorites collection")
    conn.execute("DELETE FROM collections WHERE id=?", (cid,))
    conn.commit()
    return True


def add_item(conn: sqlite3.Connection, cid: int, owner: str, sample_id: int) -> bool:
    if not _own_or_403(conn, cid, owner):
        return False
    if not conn.execute("SELECT 1 FROM samples WHERE id=?", (sample_id,)).fetchone():
        raise ValueError("no such sample")
    nextpos = conn.execute(
        "SELECT COALESCE(MAX(position),-1)+1 FROM collection_items WHERE collection_id=?", (cid,)
    ).fetchone()[0]
    conn.execute(
        "INSERT OR IGNORE INTO collection_items(collection_id,sample_id,position) VALUES(?,?,?)",
        (cid, sample_id, nextpos),
    )
    conn.commit()
    return True


def remove_item(conn: sqlite3.Connection, cid: int, owner: str, sample_id: int) -> bool:
    if not _own_or_403(conn, cid, owner):
        return False
    conn.execute(
        "DELETE FROM collection_items WHERE collection_id=? AND sample_id=?", (cid, sample_id)
    )
    conn.commit()
    return True


# ── favourites convenience (system collection) ──────────────────────────────-
def list_favorites(conn: sqlite3.Connection, owner: str) -> list[dict]:
    cid = ensure_favorites(conn, owner)
    return get_collection(conn, cid, owner)["items"]


def set_favorite(conn: sqlite3.Connection, owner: str, sample_id: int, on: bool) -> bool:
    cid = ensure_favorites(conn, owner)
    if on:
        return add_item(conn, cid, owner, sample_id)
    return remove_item(conn, cid, owner, sample_id)


def favorite_ids(conn: sqlite3.Connection, owner: str) -> list[int]:
    cid = ensure_favorites(conn, owner)
    return [
        r[0]
        for r in conn.execute(
            "SELECT sample_id FROM collection_items WHERE collection_id=?", (cid,)
        )
    ]
