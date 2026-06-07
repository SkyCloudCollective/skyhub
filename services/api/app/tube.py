"""tube.py — RanchTube: community video (embeds only, phase 1).

Members link work they host elsewhere (YouTube / Vimeo / PeerTube). We never
store or proxy the media. The route is thin (missing video -> None -> 404; bad
input -> ValueError -> 400); identity is the Matrix handle (auth.py).

SECURITY (the whole point of this module): the *raw URL is never trusted*. We
parse it ourselves, reject anything that isn't https on an allowlisted host, and
extract a bare `video_ref` with a strict per-provider regex. Only the validated
`(provider, video_ref)` is persisted. The embed URL is rebuilt — both here (for
the API response) and app-side (for the <iframe src>) — from that pair, so an
attacker-controlled string can never be reflected into an iframe/src or href
(anti iframe-injection / XSS).
"""
from __future__ import annotations

import re
import sqlite3
from urllib.parse import urlparse

from . import notify

MAX_TITLE_LEN = 160
MAX_DESC_LEN = 2000
MAX_CATEGORY_LEN = 40

PROVIDERS = ("youtube", "vimeo", "peertube")

# A video_ref, once extracted, is one of these shapes only. We re-validate the
# stored value on the way out too, so a tampered DB row still can't inject.
_YOUTUBE_ID = re.compile(r"^[A-Za-z0-9_-]{11}$")
_VIMEO_ID = re.compile(r"^[0-9]{6,12}$")
# PeerTube is federated: ref = "host/short-uuid". Host is a bare domain; the
# short-uuid is PeerTube's url-safe id. No scheme, no path traversal, no port.
_PT_HOST = re.compile(r"^[A-Za-z0-9.-]{3,253}$")
_PT_ID = re.compile(r"^[A-Za-z0-9_-]{6,64}$")

_YOUTUBE_HOSTS = {"youtube.com", "www.youtube.com", "m.youtube.com", "youtu.be"}
_VIMEO_HOSTS = {"vimeo.com", "www.vimeo.com", "player.vimeo.com"}


def _host(netloc: str) -> str:
    # strip userinfo + port; lowercase
    return netloc.rsplit("@", 1)[-1].split(":", 1)[0].strip().lower()


def parse_embed(raw_url: str) -> tuple[str, str]:
    """Validate a watch URL → (provider, video_ref). Raise ValueError on reject.

    Rejects non-https, unknown hosts, and anything whose id doesn't match the
    strict per-provider shape. Never returns the input verbatim.
    """
    raw = (raw_url or "").strip()
    if not raw:
        raise ValueError("url required")
    try:
        u = urlparse(raw)
    except ValueError:
        raise ValueError("bad url")
    if u.scheme != "https":
        raise ValueError("url must be https")
    host = _host(u.netloc)
    if not host:
        raise ValueError("bad url host")

    # ── YouTube ──────────────────────────────────────────────────────────────
    if host in _YOUTUBE_HOSTS:
        vid = ""
        if host == "youtu.be":
            vid = u.path.lstrip("/").split("/", 1)[0]
        elif u.path == "/watch":
            vid = _query_first(u.query, "v")
        elif u.path.startswith(("/embed/", "/shorts/", "/live/")):
            vid = u.path.split("/", 2)[2].split("/", 1)[0]
        if not _YOUTUBE_ID.match(vid):
            raise ValueError("could not read a YouTube video id")
        return "youtube", vid

    # ── Vimeo ────────────────────────────────────────────────────────────────
    if host in _VIMEO_HOSTS:
        # /<id>  or  player.vimeo.com/video/<id>
        parts = [p for p in u.path.split("/") if p]
        vid = parts[-1] if parts else ""
        if not _VIMEO_ID.match(vid):
            raise ValueError("could not read a Vimeo video id")
        return "vimeo", vid

    # ── PeerTube (federated; any instance) ─────────────────────────────────────
    # Recognised shapes: /w/<id>  or  /videos/watch/<id>  or  /videos/embed/<id>
    parts = [p for p in u.path.split("/") if p]
    pt_id = ""
    if len(parts) == 2 and parts[0] == "w":
        pt_id = parts[1]
    elif len(parts) == 3 and parts[0] == "videos" and parts[1] in ("watch", "embed"):
        pt_id = parts[2]
    if pt_id and _PT_HOST.match(host) and _PT_ID.match(pt_id):
        return "peertube", f"{host}/{pt_id}"

    raise ValueError("unsupported provider — use YouTube, Vimeo, or a PeerTube link")


def _query_first(query: str, key: str) -> str:
    for pair in query.split("&"):
        if "=" in pair:
            k, v = pair.split("=", 1)
            if k == key:
                return v
    return ""


def watch_url(provider: str, video_ref: str) -> str:
    """Rebuild the canonical watch URL from the validated pair (never the input)."""
    if provider == "youtube":
        return f"https://www.youtube.com/watch?v={video_ref}"
    if provider == "vimeo":
        return f"https://vimeo.com/{video_ref}"
    if provider == "peertube":
        host, _, vid = video_ref.partition("/")
        return f"https://{host}/w/{vid}"
    raise ValueError("unknown provider")


def embed_url(provider: str, video_ref: str) -> str:
    """Rebuild the iframe src from the validated pair (never the input)."""
    if provider == "youtube":
        return f"https://www.youtube-nocookie.com/embed/{video_ref}"
    if provider == "vimeo":
        return f"https://player.vimeo.com/video/{video_ref}"
    if provider == "peertube":
        host, _, vid = video_ref.partition("/")
        return f"https://{host}/videos/embed/{vid}"
    raise ValueError("unknown provider")


def _valid_stored(provider: str, video_ref: str) -> bool:
    """Defence in depth: re-check a stored pair before reflecting it."""
    if provider == "youtube":
        return bool(_YOUTUBE_ID.match(video_ref))
    if provider == "vimeo":
        return bool(_VIMEO_ID.match(video_ref))
    if provider == "peertube":
        host, _, vid = video_ref.partition("/")
        return bool(_PT_HOST.match(host) and _PT_ID.match(vid))
    return False


def _row(r: sqlite3.Row) -> dict:
    d = {k: r[k] for k in r.keys()}
    provider, video_ref = d["provider"], d["video_ref"]
    # Only ever hand the client an embed/watch URL we rebuilt from a re-validated
    # pair. A bad row (shouldn't happen) is surfaced without an embed.
    if _valid_stored(provider, video_ref):
        d["embed_url"] = embed_url(provider, video_ref)
        d["watch_url"] = watch_url(provider, video_ref)
    else:
        d["embed_url"] = None
        d["watch_url"] = None
    return d


# ── reads ─────────────────────────────────────────────────────────────────────
def list_videos(
    conn: sqlite3.Connection,
    *,
    handle: str | None = None,
    category: str | None = None,
    limit: int = 60,
) -> list[dict]:
    limit = max(1, min(int(limit or 60), 200))
    sql = (
        "SELECT id, handle, title, provider, video_ref, url, description, category, created_at "
        "FROM videos"
    )
    where: list[str] = []
    args: list = []
    if handle:
        where.append("handle=?")
        args.append(handle)
    if category:
        where.append("category=?")
        args.append(category)
    if where:
        sql += " WHERE " + " AND ".join(where)
    sql += " ORDER BY created_at DESC, id DESC LIMIT ?"
    args.append(limit)
    return [_row(r) for r in conn.execute(sql, args).fetchall()]


def get_video(conn: sqlite3.Connection, video_id: int) -> dict | None:
    r = conn.execute(
        "SELECT id, handle, title, provider, video_ref, url, description, category, created_at "
        "FROM videos WHERE id=?",
        (video_id,),
    ).fetchone()
    return _row(r) if r else None


# ── write ───────────────────────────────────────────────────────────────────-
def create_video(
    conn: sqlite3.Connection,
    handle: str,
    *,
    url: str,
    title: str,
    description: str | None = None,
    category: str | None = None,
) -> dict:
    title = (title or "").strip()
    if not title:
        raise ValueError("title required")
    if len(title) > MAX_TITLE_LEN:
        raise ValueError("title too long")
    description = (description or "").strip() or None
    if description and len(description) > MAX_DESC_LEN:
        raise ValueError("description too long")
    category = (category or "").strip() or None
    if category and len(category) > MAX_CATEGORY_LEN:
        raise ValueError("category too long")

    provider, video_ref = parse_embed(url)  # raises ValueError on reject
    canonical = watch_url(provider, video_ref)  # store our rebuilt URL, not the input

    cur = conn.execute(
        "INSERT INTO videos(handle, title, provider, video_ref, url, description, category) "
        "VALUES(?,?,?,?,?,?,?)",
        (handle, title, provider, video_ref, canonical, description, category),
    )
    # Tell followers their contributor posted (best-effort; no self-notify).
    notify.add(
        conn, handle, "system", handle,
        subject_type="video", subject_id=cur.lastrowid, data={"title": title, "kind": "tube"},
    )
    conn.commit()
    return get_video(conn, cur.lastrowid)
