"""Comments + emoji reactions on samples (as dev handle 'me'); a direct-module
test covers the not-the-author delete path that single-user HTTP can't."""
from __future__ import annotations

import pytest

from app import social


def _sample_id(client) -> int:
    return client.get("/v1/search").json()["hits"][0]["id"]


def test_comment_thread_lifecycle(client):
    sid = _sample_id(client)
    r = client.post(f"/v1/sample/{sid}/comments", json={"body": "fat kick"})
    assert r.status_code == 201
    c = r.json()
    assert c["handle"] == "me" and c["body"] == "fat kick" and c["parent_id"] is None

    reply = client.post(f"/v1/sample/{sid}/comments", json={"body": "agreed", "parent_id": c["id"]})
    assert reply.status_code == 201 and reply.json()["parent_id"] == c["id"]

    lst = client.get(f"/v1/sample/{sid}/comments").json()
    assert len(lst) == 2 and lst[0]["id"] == c["id"]

    assert client.delete(f"/v1/sample/{sid}/comments/{c['id']}").status_code == 204


def test_comment_empty_is_400(client):
    sid = _sample_id(client)
    assert client.post(f"/v1/sample/{sid}/comments", json={"body": "   "}).status_code == 400


def test_comment_unknown_sample_404(client):
    assert client.post("/v1/sample/9999/comments", json={"body": "hi"}).status_code == 404


def test_reactions_toggle(client):
    sid = _sample_id(client)
    r = client.post(f"/v1/sample/{sid}/reactions", json={"emoji": "🔥"}).json()
    assert r["counts"].get("🔥") == 1 and "🔥" in r["mine"]

    g = client.get(f"/v1/sample/{sid}/reactions").json()
    assert g["counts"].get("🔥") == 1

    r2 = client.request("DELETE", f"/v1/sample/{sid}/reactions", json={"emoji": "🔥"}).json()
    assert "🔥" not in r2["counts"] and "🔥" not in r2["mine"]


def test_reaction_bad_emoji_is_400(client):
    sid = _sample_id(client)
    assert client.post(f"/v1/sample/{sid}/reactions", json={"emoji": ""}).status_code == 400


def test_delete_others_comment_forbidden(wconn):
    sid = wconn.execute("SELECT id FROM samples LIMIT 1").fetchone()[0]
    c = social.add_comment(wconn, sid, "alice", "hi")
    with pytest.raises(PermissionError):
        social.delete_comment(wconn, c["id"], "bob")
