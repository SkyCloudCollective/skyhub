"""The public-UGC (E1) gate: 'open' project visibility is refused until the
operator sets RS_OPEN_UGC. private/unlisted are always allowed; existing open
projects stay readable."""
from __future__ import annotations


def test_me_reports_gate_off_by_default(client, monkeypatch):
    monkeypatch.delenv("RS_OPEN_UGC", raising=False)
    assert client.get("/v1/me").json()["open_ugc"] is False


def test_open_project_refused_when_gate_off(client, monkeypatch):
    monkeypatch.delenv("RS_OPEN_UGC", raising=False)
    r = client.post("/v1/projects", json={"title": "Public jam", "visibility": "open"})
    assert r.status_code == 400
    # private + unlisted still work
    assert client.post("/v1/projects", json={"title": "Mine", "visibility": "private"}).status_code == 201
    assert client.post("/v1/projects", json={"title": "Linkable", "visibility": "unlisted"}).status_code == 201


def test_open_project_allowed_when_gate_on(client, monkeypatch):
    monkeypatch.setenv("RS_OPEN_UGC", "1")
    assert client.get("/v1/me").json()["open_ugc"] is True
    r = client.post("/v1/projects", json={"title": "Public jam", "visibility": "open"})
    assert r.status_code == 201 and r.json()["visibility"] == "open"


def test_cannot_switch_to_open_when_gate_off(client, monkeypatch):
    monkeypatch.delenv("RS_OPEN_UGC", raising=False)
    p = client.post("/v1/projects", json={"title": "Mine", "visibility": "private"}).json()
    assert client.patch(f"/v1/projects/{p['id']}", json={"visibility": "open"}).status_code == 400
    assert client.patch(f"/v1/projects/{p['id']}", json={"visibility": "unlisted"}).status_code == 200
