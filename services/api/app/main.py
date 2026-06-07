"""main.py — the single RanchSamples API (FastAPI).

Replaces the v1 dual server (stdlib serve.py + FastAPI app.py kept in lockstep).
One implementation, OpenAPI docs for free, the same /v1 surface. Read endpoints
land in P0; write paths (uploads, community) follow in P1+.
"""
from __future__ import annotations

import hashlib
import mimetypes
import pathlib
import sqlite3
from contextlib import asynccontextmanager
from typing import Optional

from fastapi import Depends, FastAPI, Header, HTTPException, Query, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from pydantic import BaseModel

from . import __version__, auth, board, catalog, community, config, notify, projects, social
from .db import PREVIEWS_ROOT, PROJECTS_ROOT, SAMPLES_ROOT, connect, init_db


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


def wdb() -> sqlite3.Connection:
    """Writable connection for community write paths."""
    conn = connect()
    try:
        yield conn
    finally:
        conn.close()


def acting(authorization: Optional[str] = Header(default=None)) -> str:
    """The handle performing the request (dev mode -> the dev handle)."""
    return auth.current_handle(authorization)


# ── discovery ────────────────────────────────────────────────────────────────
@app.get("/v1/health")
def health():
    return {"status": "ok", "service": "ranchsamples-api", "version": __version__}


@app.get("/v1/me")
def me():
    return {
        "auth_enabled": auth.auth_enabled(),
        "handle": auth.current_handle(),
        "open_ugc": config.open_ugc_enabled(),
    }


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


@app.get("/v1/galaxy")
def galaxy(limit: int = Query(default=2000, ge=1, le=5000), conn: sqlite3.Connection = Depends(db)):
    return catalog.galaxy(conn, limit)


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


# ── profiles ─────────────────────────────────────────────────────────────────
class ProfilePatch(BaseModel):
    display_name: Optional[str] = None
    bio: Optional[str] = None
    links: Optional[list] = None


@app.get("/v1/profile/{handle}")
def get_profile(
    handle: str, conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)
):
    return community.get_profile(conn, auth.clean_handle(handle), viewer=me)


@app.get("/v1/me/profile")
def my_profile(conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    return community.get_profile(conn, me, viewer=me)


@app.patch("/v1/me/profile")
def patch_profile(
    body: ProfilePatch, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    return community.upsert_profile(
        conn, me, display_name=body.display_name, bio=body.bio, links=body.links
    )


# ── collections (crates / favourites / smart) ────────────────────────────────
class CollectionCreate(BaseModel):
    name: str
    visibility: str = "private"


class CollectionPatch(BaseModel):
    name: Optional[str] = None
    visibility: Optional[str] = None
    position: Optional[int] = None


class ItemAdd(BaseModel):
    sample_id: int


def _forbidden_to_http(fn):
    try:
        return fn()
    except PermissionError:
        raise HTTPException(status_code=403, detail="forbidden")
    except ValueError as e:
        raise HTTPException(status_code=400, detail=str(e))


@app.get("/v1/collections")
def list_collections(
    owner: Optional[str] = None,
    conn: sqlite3.Connection = Depends(db),
    me: str = Depends(acting),
):
    who = auth.clean_handle(owner) if owner else me
    return community.list_collections(conn, who, viewer=me)


@app.post("/v1/collections", status_code=201)
def create_collection(
    body: CollectionCreate, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    return _forbidden_to_http(
        lambda: community.create_collection(conn, me, body.name, visibility=body.visibility)
    )


@app.get("/v1/collections/{cid}")
def get_collection(
    cid: int,
    token: Optional[str] = None,
    conn: sqlite3.Connection = Depends(db),
    me: str = Depends(acting),
):
    try:
        res = community.get_collection(conn, cid, viewer=me, token=token)
    except PermissionError:
        raise HTTPException(status_code=403, detail="forbidden")
    if res is None:
        raise HTTPException(status_code=404, detail="collection not found")
    return res


@app.patch("/v1/collections/{cid}")
def patch_collection(
    cid: int, body: CollectionPatch,
    conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting),
):
    res = _forbidden_to_http(
        lambda: community.update_collection(conn, cid, me, body.model_dump(exclude_none=True))
    )
    if res is None:
        raise HTTPException(status_code=404, detail="collection not found")
    return res


@app.delete("/v1/collections/{cid}", status_code=204)
def delete_collection(
    cid: int, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    ok = _forbidden_to_http(lambda: community.delete_collection(conn, cid, me))
    if not ok:
        raise HTTPException(status_code=404, detail="collection not found")


@app.post("/v1/collections/{cid}/items", status_code=201)
def add_item(
    cid: int, body: ItemAdd, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    ok = _forbidden_to_http(lambda: community.add_item(conn, cid, me, body.sample_id))
    if not ok:
        raise HTTPException(status_code=404, detail="collection not found")
    return {"ok": True}


@app.delete("/v1/collections/{cid}/items/{sample_id}", status_code=204)
def remove_item(
    cid: int, sample_id: int, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    ok = _forbidden_to_http(lambda: community.remove_item(conn, cid, me, sample_id))
    if not ok:
        raise HTTPException(status_code=404, detail="collection not found")


# ── favourites (system collection) ───────────────────────────────────────────
class FavAdd(BaseModel):
    sample_id: int


@app.get("/v1/favorites")
def get_favorites(conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    return community.list_favorites(conn, me)


@app.get("/v1/favorites/ids")
def get_favorite_ids(conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    return community.favorite_ids(conn, me)


@app.post("/v1/favorites", status_code=201)
def add_favorite(
    body: FavAdd, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    _forbidden_to_http(lambda: community.set_favorite(conn, me, body.sample_id, True))
    return {"ok": True}


@app.delete("/v1/favorites/{sample_id}", status_code=204)
def remove_favorite(
    sample_id: int, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    community.set_favorite(conn, me, sample_id, False)


# ── collaboration: projects ("a GitHub of music") ────────────────────────────
class ProjectCreate(BaseModel):
    title: str
    kind: Optional[str] = None
    daw: Optional[str] = None
    visibility: str = "private"
    license: Optional[str] = None


class ProjectPatch(BaseModel):
    title: Optional[str] = None
    description: Optional[str] = None
    kind: Optional[str] = None
    daw: Optional[str] = None
    license: Optional[str] = None
    visibility: Optional[str] = None


class InviteCreate(BaseModel):
    handle: str
    role: str = "editor"


class PComment(BaseModel):
    body: str
    rel_path: Optional[str] = None


@app.get("/v1/projects")
def list_projects(conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    return projects.list_projects(conn, me)


@app.post("/v1/projects", status_code=201)
def create_project(
    body: ProjectCreate, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    return _forbidden_to_http(
        lambda: projects.create_project(
            conn, me, body.title, kind=body.kind, daw=body.daw,
            visibility=body.visibility, license=body.license or "all-rights-reserved",
        )
    )


@app.get("/v1/projects/{pid}")
def get_project(pid: str, conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    try:
        res = projects.get_project(conn, pid, me)
    except PermissionError:
        raise HTTPException(status_code=403, detail="forbidden")
    if res is None:
        raise HTTPException(status_code=404, detail="project not found")
    return res


@app.patch("/v1/projects/{pid}")
def patch_project(
    pid: str, body: ProjectPatch,
    conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting),
):
    res = _forbidden_to_http(
        lambda: projects.update_project(conn, pid, me, body.model_dump(exclude_none=True))
    )
    if res is None:
        raise HTTPException(status_code=404, detail="project not found")
    return res


@app.delete("/v1/projects/{pid}", status_code=204)
def delete_project(pid: str, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)):
    ok = _forbidden_to_http(lambda: projects.delete_project(conn, pid, me))
    if not ok:
        raise HTTPException(status_code=404, detail="project not found")


# members & invitations
@app.post("/v1/projects/{pid}/invite", status_code=201)
def invite_member(
    pid: str, body: InviteCreate,
    conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting),
):
    res = _forbidden_to_http(lambda: projects.invite(conn, pid, me, body.handle, body.role))
    if res is None:
        raise HTTPException(status_code=404, detail="project not found")
    return res


@app.get("/v1/invites")
def my_invites(conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    return projects.my_invites(conn, me)


@app.post("/v1/invites/{invite_id}/accept")
def accept_invite(
    invite_id: int, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    if not projects.respond_invite(conn, invite_id, me, True):
        raise HTTPException(status_code=404, detail="invite not found")
    return {"ok": True}


@app.post("/v1/invites/{invite_id}/decline")
def decline_invite(
    invite_id: int, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    if not projects.respond_invite(conn, invite_id, me, False):
        raise HTTPException(status_code=404, detail="invite not found")
    return {"ok": True}


@app.post("/v1/projects/{pid}/join", status_code=201)
def join_project(pid: str, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)):
    ok = _forbidden_to_http(lambda: projects.join_open(conn, pid, me))
    if not ok:
        raise HTTPException(status_code=404, detail="project not found")
    return {"ok": True}


@app.delete("/v1/projects/{pid}/members/{handle}", status_code=204)
def remove_member(
    pid: str, handle: str, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    ok = _forbidden_to_http(lambda: projects.remove_member(conn, pid, me, handle))
    if not ok:
        raise HTTPException(status_code=404, detail="project not found")


# files (local stub backend; a Nextcloud group folder in prod)
def _project_file_path(pid_dir: str, rel_path: str) -> pathlib.Path:
    base = (PROJECTS_ROOT / pid_dir).resolve()
    target = (base / rel_path).resolve()
    if base != target and base not in target.parents:
        raise HTTPException(status_code=400, detail="bad path")
    return target


@app.put("/v1/projects/{pid}/files/{rel_path:path}", status_code=201)
async def upload_file(
    pid: str, rel_path: str, request: Request,
    kind: Optional[str] = None, me: str = Depends(acting),
):
    # async endpoint: open the DB connection in THIS thread (sqlite is thread-bound).
    data = await request.body()
    conn = connect()
    try:
        row = projects._project_row(conn, pid)
        if not row:
            raise HTTPException(status_code=404, detail="project not found")
        path = _project_file_path(str(row["id"]), rel_path)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        sha = hashlib.sha256(data).hexdigest()
        res = _forbidden_to_http(
            lambda: projects.register_file(
                conn, pid, me, rel_path, kind=kind, bytes_=len(data), sha256=sha
            )
        )
        if res is None:
            raise HTTPException(status_code=404, detail="project not found")
        return res
    finally:
        conn.close()


@app.get("/v1/projects/{pid}/files/{rel_path:path}")
def download_file(
    pid: str, rel_path: str, conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)
):
    try:
        proj = projects.get_project(conn, pid, me)  # view-permission check
    except PermissionError:
        raise HTTPException(status_code=403, detail="forbidden")
    if proj is None:
        raise HTTPException(status_code=404, detail="project not found")
    path = _project_file_path(str(proj["project"]["id"]), rel_path)
    if not path.is_file():
        raise HTTPException(status_code=404, detail="file missing")
    return FileResponse(
        path,
        media_type="application/octet-stream",
        filename=pathlib.Path(rel_path).name,
    )


@app.delete("/v1/projects/{pid}/files/{rel_path:path}", status_code=204)
def delete_file(
    pid: str, rel_path: str, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    row = projects._project_row(conn, pid)
    if not row:
        raise HTTPException(status_code=404, detail="project not found")
    ok = _forbidden_to_http(lambda: projects.remove_file(conn, pid, me, rel_path))
    if not ok:
        raise HTTPException(status_code=404, detail="project not found")
    p = _project_file_path(str(row["id"]), rel_path)
    if p.is_file():
        p.unlink()


# comments
@app.get("/v1/projects/{pid}/comments")
def project_comments(pid: str, conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    return projects.list_comments(conn, pid)


@app.post("/v1/projects/{pid}/comments", status_code=201)
def add_project_comment(
    pid: str, body: PComment,
    conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting),
):
    res = _forbidden_to_http(
        lambda: projects.add_comment(conn, pid, me, body.body, rel_path=body.rel_path)
    )
    if res is None:
        raise HTTPException(status_code=404, detail="project not found")
    return res


# ── transparency: roadmap, feature requests + votes, feedback ────────────────-
class FeatureRequestCreate(BaseModel):
    title: str
    body: Optional[str] = None


class FeedbackCreate(BaseModel):
    message: str
    context: Optional[str] = None


@app.get("/v1/roadmap")
def get_roadmap(conn: sqlite3.Connection = Depends(db)):
    return board.list_roadmap(conn)


@app.get("/v1/feature-requests")
def list_feature_requests(
    status: Optional[str] = Query(default=None),
    conn: sqlite3.Connection = Depends(db),
    me: str = Depends(acting),
):
    return board.list_feature_requests(conn, me, status)


@app.post("/v1/feature-requests", status_code=201)
def create_feature_request(
    body: FeatureRequestCreate,
    conn: sqlite3.Connection = Depends(wdb),
    me: str = Depends(acting),
):
    return _forbidden_to_http(lambda: board.create_feature_request(conn, me, body.title, body.body))


@app.post("/v1/feature-requests/{rid}/vote")
def upvote_feature(
    rid: int, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    res = board.vote(conn, rid, me, True)
    if res is None:
        raise HTTPException(status_code=404, detail="request not found")
    return res


@app.delete("/v1/feature-requests/{rid}/vote")
def unvote_feature(
    rid: int, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)
):
    res = board.vote(conn, rid, me, False)
    if res is None:
        raise HTTPException(status_code=404, detail="request not found")
    return res


@app.post("/v1/feedback", status_code=201)
def submit_feedback(
    body: FeedbackCreate,
    conn: sqlite3.Connection = Depends(wdb),
    me: str = Depends(acting),
):
    return _forbidden_to_http(lambda: board.submit_feedback(conn, me, body.message, body.context))


# ── social: comments + reactions on samples ──────────────────────────────────
class CommentCreate(BaseModel):
    body: str
    parent_id: Optional[int] = None


class ReactionToggle(BaseModel):
    emoji: str


@app.get("/v1/sample/{sample_id}/comments")
def sample_comments(sample_id: int, conn: sqlite3.Connection = Depends(db)):
    return social.list_comments(conn, sample_id)


@app.post("/v1/sample/{sample_id}/comments", status_code=201)
def add_sample_comment(
    sample_id: int,
    body: CommentCreate,
    conn: sqlite3.Connection = Depends(wdb),
    me: str = Depends(acting),
):
    res = _forbidden_to_http(
        lambda: social.add_comment(conn, sample_id, me, body.body, parent_id=body.parent_id)
    )
    if res is None:
        raise HTTPException(status_code=404, detail="sample not found")
    return res


@app.delete("/v1/sample/{sample_id}/comments/{comment_id}", status_code=204)
def delete_sample_comment(
    sample_id: int,
    comment_id: int,
    conn: sqlite3.Connection = Depends(wdb),
    me: str = Depends(acting),
):
    res = _forbidden_to_http(lambda: social.delete_comment(conn, comment_id, me))
    if res is None:
        raise HTTPException(status_code=404, detail="comment not found")


@app.get("/v1/sample/{sample_id}/reactions")
def sample_reactions(
    sample_id: int, conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)
):
    return social.reactions(conn, sample_id, me)


@app.post("/v1/sample/{sample_id}/reactions")
def add_sample_reaction(
    sample_id: int,
    body: ReactionToggle,
    conn: sqlite3.Connection = Depends(wdb),
    me: str = Depends(acting),
):
    res = _forbidden_to_http(lambda: social.toggle_reaction(conn, sample_id, me, body.emoji, True))
    if res is None:
        raise HTTPException(status_code=404, detail="sample not found")
    return res


@app.delete("/v1/sample/{sample_id}/reactions")
def remove_sample_reaction(
    sample_id: int,
    body: ReactionToggle,
    conn: sqlite3.Connection = Depends(wdb),
    me: str = Depends(acting),
):
    res = _forbidden_to_http(lambda: social.toggle_reaction(conn, sample_id, me, body.emoji, False))
    if res is None:
        raise HTTPException(status_code=404, detail="sample not found")
    return res


# ── follows + feed + notifications ───────────────────────────────────────────-
@app.post("/v1/follow/{handle}")
def follow(handle: str, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)):
    return _forbidden_to_http(
        lambda: community.set_follow(conn, me, auth.clean_handle(handle), True)
    )


@app.delete("/v1/follow/{handle}")
def unfollow(handle: str, conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)):
    return _forbidden_to_http(
        lambda: community.set_follow(conn, me, auth.clean_handle(handle), False)
    )


@app.get("/v1/me/following")
def my_following(conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    return community.following_of(conn, me)


@app.get("/v1/feed")
def my_feed(conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    return community.feed(conn, me)


@app.get("/v1/notifications")
def get_notifications(conn: sqlite3.Connection = Depends(db), me: str = Depends(acting)):
    return notify.list_for(conn, me)


@app.post("/v1/notifications/read")
def read_notifications(conn: sqlite3.Connection = Depends(wdb), me: str = Depends(acting)):
    return notify.mark_all_read(conn, me)
