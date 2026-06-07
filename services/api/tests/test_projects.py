"""Collaboration: projects, in-app invitations, files index, activity, comments.

HTTP runs as the dev handle 'me'; multi-user flows (invite/accept, visibility,
roles) are exercised directly against the DB so a second handle exists."""
from __future__ import annotations

import pytest

from app import projects


@pytest.fixture(autouse=True)
def _open_ugc(monkeypatch):
    # this module exercises the open-collaboration feature, so enable the gate
    monkeypatch.setenv("RS_OPEN_UGC", "1")


# ── HTTP (as 'me') ───────────────────────────────────────────────────────────
def test_create_list_get_project(client):
    pid = client.post("/v1/projects", json={"title": "Night Drive", "daw": "ableton"}).json()["id"]
    listed = client.get("/v1/projects").json()
    assert any(p["id"] == pid and p["title"] == "Night Drive" for p in listed)
    full = client.get(f"/v1/projects/{pid}").json()
    assert full["project"]["slug"] == "night-drive"
    assert full["role"] == "owner"
    assert [m["handle"] for m in full["members"]] == ["me"]
    # creation is logged
    assert any(a["action"] == "create" for a in full["activity"])


def test_get_by_slug(client):
    client.post("/v1/projects", json={"title": "Sunset Tape"})
    full = client.get("/v1/projects/sunset-tape").json()
    assert full["project"]["title"] == "Sunset Tape"


def test_upload_index_download_file(client):
    pid = client.post("/v1/projects", json={"title": "Stems"}).json()["id"]
    r = client.put(f"/v1/projects/{pid}/files/stems/bass.wav?kind=stem", content=b"RIFFxxxxWAVE")
    assert r.status_code == 201
    assert r.json()["version"] == 1
    full = client.get(f"/v1/projects/{pid}").json()
    assert any(f["rel_path"] == "stems/bass.wav" and f["kind"] == "stem" for f in full["files"])
    # re-upload bumps the version
    assert client.put(f"/v1/projects/{pid}/files/stems/bass.wav", content=b"RIFFyyyy").json()["version"] == 2
    # download returns the bytes
    dl = client.get(f"/v1/projects/{pid}/files/stems/bass.wav")
    assert dl.status_code == 200 and dl.content == b"RIFFyyyy"
    # delete
    assert client.delete(f"/v1/projects/{pid}/files/stems/bass.wav").status_code == 204
    assert client.get(f"/v1/projects/{pid}/files/stems/bass.wav").status_code == 404


def test_comments(client):
    pid = client.post("/v1/projects", json={"title": "Chat"}).json()["id"]
    client.post(f"/v1/projects/{pid}/comments", json={"body": "intro is too long"})
    cs = client.get(f"/v1/projects/{pid}/comments").json()
    assert cs[0]["body"] == "intro is too long"


def test_delete_project(client):
    pid = client.post("/v1/projects", json={"title": "Temp"}).json()["id"]
    assert client.delete(f"/v1/projects/{pid}").status_code == 204
    assert client.get(f"/v1/projects/{pid}").status_code == 404


def test_path_traversal_blocked(client):
    pid = client.post("/v1/projects", json={"title": "Safe"}).json()["id"]
    r = client.put(f"/v1/projects/{pid}/files/../escape.txt", content=b"x")
    assert r.status_code in (400, 404)


# ── multi-user (direct) ──────────────────────────────────────────────────────
def test_private_hidden_from_non_member(wconn):
    p = projects.create_project(wconn, "alice", "Secret EP", visibility="private")
    assert all(x["id"] != p["id"] for x in projects.list_projects(wconn, "bob"))
    with pytest.raises(PermissionError):
        projects.get_project(wconn, p["id"], "bob")
    assert projects.get_project(wconn, p["id"], "alice")["role"] == "owner"


def test_invite_accept_flow_and_activity(wconn):
    p = projects.create_project(wconn, "alice", "Collab", visibility="private")
    inv = projects.invite(wconn, p["id"], "alice", "bob", role="editor")
    assert inv["status"] == "pending"
    # bob sees it and the project is still hidden until accepted
    assert projects.my_invites(wconn, "bob")[0]["title"] == "Collab"
    with pytest.raises(PermissionError):
        projects.get_project(wconn, p["id"], "bob")
    assert projects.respond_invite(wconn, inv["id"], "bob", True) is True
    # now bob is a member and can see it
    assert projects.role_of(wconn, p["id"], "bob") == "editor"
    full = projects.get_project(wconn, p["id"], "bob")
    assert full["role"] == "editor"
    assert any(a["action"] == "join" and a["actor"] == "bob" for a in full["activity"])


def test_open_project_free_join(wconn):
    p = projects.create_project(wconn, "alice", "Open Jam", visibility="open")
    # listed to everyone, joinable without an invite
    assert any(x["id"] == p["id"] for x in projects.list_projects(wconn, "carol"))
    assert projects.join_open(wconn, p["id"], "carol") is True
    assert projects.role_of(wconn, p["id"], "carol") == "editor"


def test_non_member_cannot_join_private(wconn):
    p = projects.create_project(wconn, "alice", "Closed", visibility="private")
    with pytest.raises(PermissionError):
        projects.join_open(wconn, p["id"], "bob")


def test_editor_only_file_register(wconn):
    p = projects.create_project(wconn, "alice", "Files", visibility="open")
    # carol is not a member yet → cannot register a file
    with pytest.raises(PermissionError):
        projects.register_file(wconn, p["id"], "carol", "loop.wav", kind="loop")
    projects.join_open(wconn, p["id"], "carol")
    assert projects.register_file(wconn, p["id"], "carol", "loop.wav", kind="loop")["version"] == 1


def test_owner_only_invite_and_delete(wconn):
    p = projects.create_project(wconn, "alice", "Mine", visibility="open")
    projects.join_open(wconn, p["id"], "bob")  # bob is an editor
    # an editor CAN invite (collaborative), a non-member cannot
    assert projects.invite(wconn, p["id"], "bob", "dave")["status"] == "pending"
    with pytest.raises(PermissionError):
        projects.invite(wconn, p["id"], "stranger", "eve")
    # only the owner can delete
    with pytest.raises(PermissionError):
        projects.delete_project(wconn, p["id"], "bob")
    assert projects.delete_project(wconn, p["id"], "alice") is True
