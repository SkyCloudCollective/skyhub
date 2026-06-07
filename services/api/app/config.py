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
