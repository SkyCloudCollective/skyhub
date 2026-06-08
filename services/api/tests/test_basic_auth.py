"""The HTTP Basic gate (RS_BASIC_AUTH) + the X-RS-Handle identity header.

The gate is a coarse front-door lock for a protected preview deploy: when the
env is set, every route needs Basic creds except the health probe; when unset,
no gate at all. Identity rides on X-RS-Handle so it never collides with the
Authorization header the gate uses.
"""
from __future__ import annotations

import base64


def _basic(user: str, pw: str) -> dict:
    token = base64.b64encode(f"{user}:{pw}".encode()).decode()
    return {"Authorization": f"Basic {token}"}


# ── gate ──────────────────────────────────────────────────────────────────────
def test_no_gate_when_env_absent(client, monkeypatch):
    monkeypatch.delenv("RS_BASIC_AUTH", raising=False)
    assert client.get("/v1/me").status_code == 200


def test_gate_blocks_without_creds(client, monkeypatch):
    monkeypatch.setenv("RS_BASIC_AUTH", "scout:letmein")
    r = client.get("/v1/me")
    assert r.status_code == 401
    assert r.headers.get("www-authenticate") == 'Basic realm="SkyHub"'


def test_gate_allows_with_creds(client, monkeypatch):
    monkeypatch.setenv("RS_BASIC_AUTH", "scout:letmein")
    assert client.get("/v1/me", headers=_basic("scout", "letmein")).status_code == 200


def test_gate_rejects_wrong_creds(client, monkeypatch):
    monkeypatch.setenv("RS_BASIC_AUTH", "scout:letmein")
    assert client.get("/v1/me", headers=_basic("scout", "nope")).status_code == 401
    assert client.get("/v1/me", headers=_basic("mallory", "letmein")).status_code == 401


def test_healthz_is_always_open(client, monkeypatch):
    monkeypatch.setenv("RS_BASIC_AUTH", "scout:letmein")
    assert client.get("/healthz").status_code == 200


def test_gate_malformed_header_is_401(client, monkeypatch):
    monkeypatch.setenv("RS_BASIC_AUTH", "scout:letmein")
    assert client.get("/v1/me", headers={"Authorization": "Basic not-base64!!"}).status_code == 401
    assert client.get("/v1/me", headers={"Authorization": "Bearer abc"}).status_code == 401


# ── X-RS-Handle identity (priority over Authorization) ────────────────────────
def test_x_rs_handle_sets_acting_identity(client):
    r = client.get("/v1/me", headers={"X-RS-Handle": "luna"})
    assert r.json()["handle"] == "luna"


def test_x_rs_handle_wins_under_the_gate(client, monkeypatch):
    # Authorization carries the Basic creds; identity still comes from X-RS-Handle.
    monkeypatch.setenv("RS_BASIC_AUTH", "scout:letmein")
    r = client.get(
        "/v1/me",
        headers={**_basic("scout", "letmein"), "X-RS-Handle": "luna"},
    )
    assert r.status_code == 200 and r.json()["handle"] == "luna"


def test_x_rs_handle_drives_authorship(client):
    # a video posted as 'luna' is attributed to luna, not the dev handle
    r = client.post(
        "/v1/tube",
        json={"url": "https://youtu.be/dQw4w9WgXcQ", "title": "luna's set"},
        headers={"X-RS-Handle": "luna"},
    )
    assert r.status_code == 201 and r.json()["handle"] == "luna"
    assert client.get("/v1/tube?handle=luna").json()[0]["title"] == "luna's set"


def test_x_rs_handle_is_cleaned(client):
    # '@Luna:server' -> 'luna' (matches catalog identity normalisation)
    assert client.get("/v1/me", headers={"X-RS-Handle": "@Luna:server.tld"}).json()["handle"] == "luna"


def test_absent_x_rs_handle_keeps_dev_default(client):
    assert client.get("/v1/me").json()["handle"] == "me"


# ── per-member Matrix gate (each member signs in with their own PDF creds) ─────
def _enable_matrix(monkeypatch, validate):
    """Turn on the per-member gate; `validate(user, pw)` stands in for Synapse."""
    from app import auth, main

    main._AUTH_CACHE.clear()
    monkeypatch.setenv("RANCHSAMPLES_MATRIX_HS", "https://music.example.test")
    monkeypatch.delenv("RS_BASIC_AUTH", raising=False)
    monkeypatch.setattr(auth, "matrix_login", validate)


def test_matrix_gate_allows_a_valid_member(client, monkeypatch):
    _enable_matrix(monkeypatch, lambda u, pw: "lionel" if (u, pw) == ("lionel", "pdf-pass") else None)
    r = client.get("/v1/me", headers=_basic("lionel", "pdf-pass"))
    assert r.status_code == 200
    # identity is the signed-in member — not the dev handle, not a client header
    assert r.json()["handle"] == "lionel"
    assert r.json()["auth_enabled"] is True


def test_matrix_gate_rejects_bad_password(client, monkeypatch):
    _enable_matrix(monkeypatch, lambda u, pw: "lionel" if pw == "pdf-pass" else None)
    assert client.get("/v1/me", headers=_basic("lionel", "wrong")).status_code == 401


def test_matrix_gate_requires_creds(client, monkeypatch):
    _enable_matrix(monkeypatch, lambda u, pw: "lionel")
    r = client.get("/v1/me")
    assert r.status_code == 401
    assert r.headers.get("www-authenticate") == 'Basic realm="SkyHub"'


def test_matrix_gate_identity_overrides_x_rs_handle(client, monkeypatch):
    # the authenticated member wins over any client-supplied X-RS-Handle
    _enable_matrix(monkeypatch, lambda u, pw: "lionel")
    r = client.get("/v1/me", headers={**_basic("lionel", "pdf-pass"), "X-RS-Handle": "mallory"})
    assert r.status_code == 200 and r.json()["handle"] == "lionel"


def test_matrix_gate_caches_validation(client, monkeypatch):
    calls = {"n": 0}

    def validate(u, pw):
        calls["n"] += 1
        return "lionel"

    _enable_matrix(monkeypatch, validate)
    h = _basic("lionel", "pdf-pass")
    assert client.get("/v1/me", headers=h).status_code == 200
    assert client.get("/v1/me", headers=h).status_code == 200
    assert calls["n"] == 1  # the second request is served from the cache


def test_matrix_gate_healthz_still_open(client, monkeypatch):
    _enable_matrix(monkeypatch, lambda u, pw: None)  # nobody can log in
    assert client.get("/healthz").status_code == 200
    assert client.get("/v1/health").status_code == 200
