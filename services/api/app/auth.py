"""auth.py — identity helpers (Matrix handle + JWT), kept minimal for P0.

Auth is DORMANT until RANCHSAMPLES_MATRIX_HS is set: with no homeserver the API
runs open (dev), `current_handle` returns the dev handle. Login/verify against a
real homeserver lands when the community write paths do (P3). Admin handles come
from RS_ADMIN_HANDLES (comma-separated), normalised to clean localparts.
"""
from __future__ import annotations

import json
import os
import re
import urllib.error
import urllib.request

_DEV_HANDLE = os.environ.get("RANCHSAMPLES_DEV_HANDLE", "me")


def auth_enabled() -> bool:
    return bool(os.environ.get("RANCHSAMPLES_MATRIX_HS"))


def matrix_login(user: str, password: str, homeserver: str | None = None) -> str | None:
    """Validate (user, password) against the configured Matrix homeserver (Synapse).

    Returns the member's clean handle (localpart) on success, else None. Matrix is the
    source of identity, so the per-member front-door gate (see main.basic_auth_gate)
    can let each member in with their own credentials — the same login as their PDF
    sheet. Never logs the password. Blocking (urllib) — call it off the event loop.
    """
    hs = (homeserver or os.environ.get("RANCHSAMPLES_MATRIX_HS", "")).rstrip("/")
    if not hs or not user or not password:
        return None
    body = json.dumps({
        "type": "m.login.password",
        "identifier": {"type": "m.id.user", "user": user},
        "password": password,
    }).encode()
    req = urllib.request.Request(
        hs + "/_matrix/client/v3/login", data=body, method="POST",
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as r:
            data = json.loads(r.read().decode())
    except (urllib.error.HTTPError, urllib.error.URLError, ValueError, OSError):
        return None
    handle = clean_handle(data.get("user_id") or user)
    return handle or None


def clean_handle(raw: str | None) -> str:
    """Reduce '@alice:server.tld' / 'Alice' -> 'alice' (localpart, lowercase)."""
    if not raw:
        return ""
    s = raw.strip().lstrip("@")
    s = s.split(":", 1)[0]
    s = re.sub(r"[^A-Za-z0-9._-]", "", s)
    return s.lower()


def admin_handles() -> set[str]:
    return {clean_handle(h) for h in os.environ.get("RS_ADMIN_HANDLES", "").split(",") if h.strip()}


def is_admin(handle: str | None) -> bool:
    if not auth_enabled():
        return True  # dev mode: open
    return clean_handle(handle) in admin_handles()


def current_handle(authorization: str | None = None, app_handle: str | None = None) -> str:
    """Resolve the acting handle. Dev mode -> the dev handle. (JWT verify: P3.)

    `app_handle` is the app identity header (X-RS-Handle). It takes PRIORITY over
    the Authorization header: the gated preview puts HTTP Basic creds in
    Authorization, so identity must come from a header that doesn't collide with
    it. When auth is enabled the app handle still wins (until JWT verification
    lands in P3); when it's absent we keep today's behaviour exactly.
    """
    if app_handle:
        cleaned = clean_handle(app_handle)
        if cleaned:
            return cleaned
    if not auth_enabled():
        return _DEV_HANDLE
    # P3: verify Bearer JWT here and return its subject.
    return clean_handle(authorization)
