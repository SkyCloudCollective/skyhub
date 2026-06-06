"""Profiles + collections + favourites — HTTP (as dev handle 'me') + direct
visibility unit tests (multi-viewer, which dev single-user HTTP can't cover)."""
from __future__ import annotations

import pytest

from app import community


def _a_sample_id(client) -> int:
    return client.get("/v1/search", params={"category": "loops"}).json()["hits"][0]["id"]


# ── profiles ─────────────────────────────────────────────────────────────────
def test_profile_default_then_patch(client):
    p = client.get("/v1/profile/me").json()
    assert p["handle"] == "me"
    assert p["sample_count"] == 0  # 'me' contributed nothing in the seed

    r = client.patch("/v1/me/profile", json={"display_name": "Me!", "bio": "hi"})
    assert r.status_code == 200
    p2 = client.get("/v1/profile/me").json()
    assert p2["display_name"] == "Me!"
    assert p2["bio"] == "hi"


def test_profile_sample_count_reflects_contributions(client):
    # 'tev' contributed one seed sample
    assert client.get("/v1/profile/tev").json()["sample_count"] == 1


# ── collections ──────────────────────────────────────────────────────────────
def test_create_list_get_collection(client):
    cid = client.post("/v1/collections", json={"name": "My crate"}).json()["id"]
    listed = client.get("/v1/collections").json()
    assert any(c["id"] == cid and c["name"] == "My crate" for c in listed)
    full = client.get(f"/v1/collections/{cid}").json()
    assert full["collection"]["id"] == cid
    assert full["items"] == []


def test_add_and_remove_item(client):
    cid = client.post("/v1/collections", json={"name": "Picks"}).json()["id"]
    sid = _a_sample_id(client)
    assert client.post(f"/v1/collections/{cid}/items", json={"sample_id": sid}).status_code == 201
    items = client.get(f"/v1/collections/{cid}").json()["items"]
    assert [i["id"] for i in items] == [sid]
    assert client.delete(f"/v1/collections/{cid}/items/{sid}").status_code == 204
    assert client.get(f"/v1/collections/{cid}").json()["items"] == []


def test_delete_collection(client):
    cid = client.post("/v1/collections", json={"name": "Temp"}).json()["id"]
    assert client.delete(f"/v1/collections/{cid}").status_code == 204
    assert client.get(f"/v1/collections/{cid}").status_code == 404


def test_add_item_unknown_sample_400(client):
    cid = client.post("/v1/collections", json={"name": "X"}).json()["id"]
    assert client.post(f"/v1/collections/{cid}/items", json={"sample_id": 9999}).status_code == 400


# ── favourites = system collection ───────────────────────────────────────────
def test_favorites_flow(client):
    sid = _a_sample_id(client)
    assert client.post("/v1/favorites", json={"sample_id": sid}).status_code == 201
    assert client.get("/v1/favorites/ids").json() == [sid]
    assert len(client.get("/v1/favorites").json()) == 1
    assert client.delete(f"/v1/favorites/{sid}").status_code == 204
    assert client.get("/v1/favorites/ids").json() == []


def test_cannot_delete_favorites_collection(client):
    client.get("/v1/favorites")  # ensures the system collection exists
    fav = next(c for c in client.get("/v1/collections").json() if c["kind"] == "favorites")
    assert client.delete(f"/v1/collections/{fav['id']}").status_code == 403


# ── visibility (direct, multi-viewer) ────────────────────────────────────────
def test_private_hidden_from_others(wconn):
    coll = community.create_collection(wconn, "alice", "Secret", visibility="private")
    assert community.list_collections(wconn, "alice", viewer="bob") == []
    assert community.list_collections(wconn, "alice", viewer="alice")
    with pytest.raises(PermissionError):
        community.get_collection(wconn, coll["id"], viewer="bob")


def test_public_is_listed_and_visible(wconn):
    coll = community.create_collection(wconn, "alice", "Open", visibility="public")
    assert any(c["id"] == coll["id"] for c in community.list_collections(wconn, "alice", viewer="bob"))
    assert community.get_collection(wconn, coll["id"], viewer="bob")["collection"]["name"] == "Open"


def test_unlisted_needs_token(wconn):
    coll = community.create_collection(wconn, "alice", "Link", visibility="private")
    updated = community.update_collection(wconn, coll["id"], "alice", {"visibility": "unlisted"})
    token = updated["share_token"]
    assert token
    # not listed to others
    assert community.list_collections(wconn, "alice", viewer="bob") == []
    # right token works, wrong token / none fails
    assert community.get_collection(wconn, coll["id"], viewer="bob", token=token)
    with pytest.raises(PermissionError):
        community.get_collection(wconn, coll["id"], viewer="bob", token="nope")
    with pytest.raises(PermissionError):
        community.get_collection(wconn, coll["id"], viewer="bob")


def test_update_delete_requires_owner(wconn):
    coll = community.create_collection(wconn, "alice", "Mine", visibility="public")
    with pytest.raises(PermissionError):
        community.update_collection(wconn, coll["id"], "bob", {"name": "hijack"})
    with pytest.raises(PermissionError):
        community.delete_collection(wconn, coll["id"], "bob")
