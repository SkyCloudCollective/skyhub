"""config.py — deployment feature flags.

Kept tiny and env-driven (same idiom as auth.py). The one that matters today is
the public-UGC gate: 'open' project visibility is the public user-generated-content
surface and MUST stay disabled until the legal/consent pass is cleared. Default
OFF; the operator flips RS_OPEN_UGC to a truthy value to enable it.
"""
from __future__ import annotations

import os

_TRUTHY = {"1", "true", "yes", "on"}


def open_ugc_enabled() -> bool:
    """Whether 'open' (public) project collaboration may be created/enabled.

    Default False: until the legal pass, the API refuses to create or switch a
    project to 'open' (existing open projects remain readable). private/unlisted
    are unaffected."""
    return os.environ.get("RS_OPEN_UGC", "").strip().lower() in _TRUTHY


# ── Hub: external self-hosted services the operator links to ──────────────────
# The catalogue is static (FOSS names, no private data); each instance URL is
# supplied at runtime via env, so the repo carries no operator host and every
# self-hoster points at their own. A service with no URL configured is hidden.
_HUB_CATALOG = [
    {"key": "jellyfin", "name": "Video", "tool": "Jellyfin",
     "desc": "Films & series — the shared media library.", "accent": "#60A5FA"},
    {"key": "navidrome", "name": "Music", "tool": "Navidrome",
     "desc": "Stream the collective's music.", "accent": "#A78BFA"},
    {"key": "seerr", "name": "Requests", "tool": "Seerr",
     "desc": "Ask for a film or a show to be added.", "accent": "#F59E0B"},
]


def hub_services() -> list[dict]:
    """Configured external services for the hub, in catalogue order.

    Each URL comes from `RS_HUB_<KEY>_URL` (e.g. RS_HUB_JELLYFIN_URL). Services
    without a URL set are omitted, so an instance only shows what it actually runs.
    """
    out = []
    for s in _HUB_CATALOG:
        url = os.environ.get(f"RS_HUB_{s['key'].upper()}_URL", "").strip()
        if url:
            out.append({**s, "url": url})
    return out
