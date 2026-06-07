"""auth.py — identity helpers (Matrix handle + JWT), kept minimal for P0.

Auth is DORMANT until RANCHSAMPLES_MATRIX_HS is set: with no homeserver the API
runs open (dev), `current_handle` returns the dev handle. Login/verify against a
real homeserver lands when the community write paths do (P3). Admin handles come
from RS_ADMIN_HANDLES (comma-separated), normalised to clean localparts.
"""
from __future__ import annotations

import os
import re

_DEV_HANDLE = os.environ.get("RANCHSAMPLES_DEV_HANDLE", "me")


def auth_enabled() -> bool:
    return bool(os.environ.get("RANCHSAMPLES_MATRIX_HS"))


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
