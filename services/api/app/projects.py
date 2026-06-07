"""projects.py — collaborative projects ("a GitHub of music").

The collaboration layer: projects, members/roles, in-app invitations, a files
index, an activity log ("git log"), and comments. Storage/sync is a separate
backend (a local folder in dev, a Nextcloud group folder in prod) — this module
owns *who can do what*, not the bytes.

Permissions raise PermissionError (→ 403); not-found returns None (→ 404).
Governance: invitations are in-app + consented (an invite is an offer the
invitee accepts); 'open' visibility is the public-UGC surface and is gated by a
deployment flag until the legal pass.
"""
from __future__ import annotations

import json
import re
import secrets
import sqlite3

VISIBILITIES = ("private", "unlisted", "open")
ROLES = ("owner", "editor", "viewer")
EDIT_ROLES = ("owner", "editor")


def _row(r: sqlite3.Row) -> dict:
    return {k: r[k] for k in r.keys()}


def slugify(title: str) -> str:
    s = re.sub(r"[^a-z0-9]+", "-", title.strip().lower()).strip("-")
    return s or "project"


def _unique_slug(conn: sqlite3.Connection, base: str) -> str:
    slug = base
    n = 2
    while conn.execute("SELECT 1 FROM collab_projects WHERE slug=?", (slug,)).fetchone():
        slug = f"{base}-{n}"
        n += 1
    return slug


def _log(conn, project_id, actor, action, target=None, data=None):
    conn.execute(
        "INSERT INTO project_activity(project_id,actor,action,target,data_json) VALUES(?,?,?,?,?)",
        (project_id, actor, action, target, json.dumps(data) if data is not None else None),
    )


def role_of(conn: sqlite3.Connection, project_id: int, handle: str | None) -> str | None:
    if not handle:
        return None
    r = conn.execute(
        "SELECT role FROM project_members WHERE project_id=? AND handle=?", (project_id, handle)
    ).fetchone()
    return r["role"] if r else None


# ── projects ─────────────────────────────────────────────────────────────────
def create_project(
    conn, owner, title, *, kind=None, daw=None, visibility="private", license="all-rights-reserved"
) -> dict:
    if visibility not in VISIBILITIES:
        raise ValueError("bad visibility")
    slug = _unique_slug(conn, slugify(title))
    cur = conn.execute(
        "INSERT INTO collab_projects(owner,title,slug,kind,daw,visibility,license) "
        "VALUES(?,?,?,?,?,?,?)",
        (owner, title.strip() or "Untitled", slug, kind, daw, visibility, license),
    )
    pid = cur.lastrowid
    conn.execute(
        "INSERT INTO project_members(project_id,handle,role,accepted_at) "
        "VALUES(?,?, 'owner', datetime('now'))",
        (pid, owner),
    )
    _log(conn, pid, owner, "create", title)
    conn.commit()
    return get_project(conn, pid, owner)["project"]


def _project_row(conn, id_or_slug):
    if isinstance(id_or_slug, int) or (isinstance(id_or_slug, str) and id_or_slug.isdigit()):
        r = conn.execute("SELECT * FROM collab_projects WHERE id=?", (int(id_or_slug),)).fetchone()
    else:
        r = conn.execute("SELECT * FROM collab_projects WHERE slug=?", (id_or_slug,)).fetchone()
    return r


def can_view(project: dict, role: str | None) -> bool:
    if role is not None:
        return True
    return project["visibility"] in ("unlisted", "open")


def list_projects(conn: sqlite3.Connection, viewer: str | None) -> list[dict]:
    """Projects the viewer is a member of + all open projects."""
    out: dict[int, dict] = {}
    if viewer:
        for r in conn.execute(
            "SELECT p.*, m.role FROM collab_projects p "
            "JOIN project_members m ON m.project_id=p.id AND m.handle=? "
            "ORDER BY p.updated_at DESC",
            (viewer,),
        ):
            d = _row(r)
            out[d["id"]] = d
    for r in conn.execute(
        "SELECT * FROM collab_projects WHERE visibility='open' ORDER BY updated_at DESC"
    ):
        d = _row(r)
        out.setdefault(d["id"], {**d, "role": role_of(conn, d["id"], viewer)})
    projects = list(out.values())
    for p in projects:
        p["members"] = conn.execute(
            "SELECT COUNT(*) FROM project_members WHERE project_id=?", (p["id"],)
        ).fetchone()[0]
        p["files"] = conn.execute(
            "SELECT COUNT(*) FROM project_files WHERE project_id=?", (p["id"],)
        ).fetchone()[0]
    return projects


def get_project(conn: sqlite3.Connection, id_or_slug, viewer: str | None) -> dict | None:
    r = _project_row(conn, id_or_slug)
    if not r:
        return None
    project = _row(r)
    role = role_of(conn, project["id"], viewer)
    if not can_view(project, role):
        raise PermissionError("not visible")
    members = [
        _row(m)
        for m in conn.execute(
            "SELECT handle, role, accepted_at FROM project_members WHERE project_id=? "
            "ORDER BY role!='owner', handle",
            (project["id"],),
        )
    ]
    files = [
        _row(f)
        for f in conn.execute(
            "SELECT rel_path, kind, bytes, sha256, version, updated_by, updated_at "
            "FROM project_files WHERE project_id=? ORDER BY rel_path",
            (project["id"],),
        )
    ]
    activity = [
        _row(a)
        for a in conn.execute(
            "SELECT actor, action, target, created_at FROM project_activity "
            "WHERE project_id=? ORDER BY id DESC LIMIT 50",
            (project["id"],),
        )
    ]
    return {"project": project, "role": role, "members": members, "files": files, "activity": activity}


def _owner_or_403(conn, pid, handle) -> dict | None:
    r = _project_row(conn, pid)
    if not r:
        return None
    if role_of(conn, r["id"], handle) != "owner":
        raise PermissionError("not owner")
    return _row(r)


def update_project(conn, pid, handle, fields) -> dict | None:
    p = _owner_or_403(conn, pid, handle)
    if not p:
        return None
    sets, args = [], []
    for col in ("title", "description", "kind", "daw", "license"):
        if fields.get(col) is not None:
            sets.append(f"{col}=?")
            args.append(fields[col])
    if fields.get("visibility") in VISIBILITIES:
        sets.append("visibility=?")
        args.append(fields["visibility"])
    if sets:
        sets.append("updated_at=datetime('now')")
        conn.execute(f"UPDATE collab_projects SET {','.join(sets)} WHERE id=?", args + [p["id"]])
        conn.commit()
    return get_project(conn, p["id"], handle)["project"]


def delete_project(conn, pid, handle) -> bool:
    p = _owner_or_403(conn, pid, handle)
    if not p:
        return False
    conn.execute("DELETE FROM collab_projects WHERE id=?", (p["id"],))
    conn.commit()
    return True


# ── members & invitations ────────────────────────────────────────────────────
def invite(conn, pid, inviter, handle, role="editor") -> dict | None:
    r = _project_row(conn, pid)
    if not r:
        return None
    if role_of(conn, r["id"], inviter) not in EDIT_ROLES:
        raise PermissionError("only members can invite")
    if role not in ROLES:
        role = "editor"
    if role_of(conn, r["id"], handle) is not None:
        raise ValueError("already a member")
    token = secrets.token_urlsafe(12)
    cur = conn.execute(
        "INSERT INTO project_invites(project_id,handle,role,token,invited_by) VALUES(?,?,?,?,?)",
        (r["id"], handle, role, token, inviter),
    )
    _log(conn, r["id"], inviter, "invite", handle)
    conn.commit()
    return {"id": cur.lastrowid, "project_id": r["id"], "handle": handle, "role": role, "status": "pending"}


def my_invites(conn, handle) -> list[dict]:
    return [
        _row(i)
        for i in conn.execute(
            "SELECT i.id, i.project_id, i.role, i.invited_by, p.title, p.slug "
            "FROM project_invites i JOIN collab_projects p ON p.id=i.project_id "
            "WHERE i.handle=? AND i.status='pending' ORDER BY i.id DESC",
            (handle,),
        )
    ]


def respond_invite(conn, invite_id, handle, accept: bool) -> bool:
    r = conn.execute(
        "SELECT * FROM project_invites WHERE id=? AND handle=? AND status='pending'",
        (invite_id, handle),
    ).fetchone()
    if not r:
        return False
    new_status = "accepted" if accept else "declined"
    conn.execute("UPDATE project_invites SET status=? WHERE id=?", (new_status, invite_id))
    if accept:
        conn.execute(
            "INSERT OR IGNORE INTO project_members(project_id,handle,role,invited_by,accepted_at) "
            "VALUES(?,?,?,?, datetime('now'))",
            (r["project_id"], handle, r["role"], r["invited_by"]),
        )
        _log(conn, r["project_id"], handle, "join", handle)
    conn.commit()
    return True


def join_open(conn, pid, handle) -> bool:
    r = _project_row(conn, pid)
    if not r:
        return False
    if r["visibility"] != "open":
        raise PermissionError("project is not open")
    if role_of(conn, r["id"], handle) is None:
        conn.execute(
            "INSERT INTO project_members(project_id,handle,role,accepted_at) "
            "VALUES(?,?, 'editor', datetime('now'))",
            (r["id"], handle),
        )
        _log(conn, r["id"], handle, "join", handle)
        conn.commit()
    return True


def remove_member(conn, pid, owner, handle) -> bool:
    p = _owner_or_403(conn, pid, owner)
    if not p:
        return False
    if handle == p["owner"]:
        raise PermissionError("cannot remove the owner")
    conn.execute("DELETE FROM project_members WHERE project_id=? AND handle=?", (p["id"], handle))
    conn.commit()
    return True


# ── files index + activity + comments ────────────────────────────────────────
def register_file(conn, pid, actor, rel_path, *, kind=None, bytes_=None, sha256=None) -> dict | None:
    r = _project_row(conn, pid)
    if not r:
        return None
    if role_of(conn, r["id"], actor) not in EDIT_ROLES:
        raise PermissionError("not an editor")
    existing = conn.execute(
        "SELECT version FROM project_files WHERE project_id=? AND rel_path=?", (r["id"], rel_path)
    ).fetchone()
    version = (existing["version"] + 1) if existing else 1
    conn.execute(
        "INSERT INTO project_files(project_id,rel_path,kind,bytes,sha256,version,updated_by) "
        "VALUES(?,?,?,?,?,?,?) ON CONFLICT(project_id,rel_path) DO UPDATE SET "
        "kind=excluded.kind, bytes=excluded.bytes, sha256=excluded.sha256, "
        "version=excluded.version, updated_by=excluded.updated_by, updated_at=datetime('now')",
        (r["id"], rel_path, kind, bytes_, sha256, version, actor),
    )
    conn.execute("UPDATE collab_projects SET updated_at=datetime('now') WHERE id=?", (r["id"],))
    _log(conn, r["id"], actor, "update" if existing else "add", rel_path, {"version": version})
    conn.commit()
    return {"rel_path": rel_path, "version": version}


def remove_file(conn, pid, actor, rel_path) -> bool:
    r = _project_row(conn, pid)
    if not r:
        return False
    if role_of(conn, r["id"], actor) not in EDIT_ROLES:
        raise PermissionError("not an editor")
    conn.execute("DELETE FROM project_files WHERE project_id=? AND rel_path=?", (r["id"], rel_path))
    _log(conn, r["id"], actor, "remove", rel_path)
    conn.commit()
    return True


def add_comment(conn, pid, handle, body, rel_path=None) -> dict | None:
    r = _project_row(conn, pid)
    if not r:
        return None
    if not can_view(_row(r), role_of(conn, r["id"], handle)):
        raise PermissionError("not visible")
    cur = conn.execute(
        "INSERT INTO project_comments(project_id,rel_path,handle,body) VALUES(?,?,?,?)",
        (r["id"], rel_path, handle, body.strip()),
    )
    _log(conn, r["id"], handle, "comment", rel_path)
    conn.commit()
    return {"id": cur.lastrowid, "handle": handle, "body": body.strip(), "rel_path": rel_path}


def list_comments(conn, pid) -> list[dict]:
    r = _project_row(conn, pid)
    if not r:
        return []
    return [
        _row(c)
        for c in conn.execute(
            "SELECT id, rel_path, handle, body, created_at FROM project_comments "
            "WHERE project_id=? ORDER BY id",
            (r["id"],),
        )
    ]
