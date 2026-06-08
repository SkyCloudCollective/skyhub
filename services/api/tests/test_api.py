"""Smoke + read-path tests for the single FastAPI service."""
from __future__ import annotations


def test_health(client):
    r = client.get("/v1/health")
    assert r.status_code == 200
    body = r.json()
    assert body["status"] == "ok"
    assert body["service"] == "skyhub-api"


def test_me_dev_mode_is_open(client):
    body = client.get("/v1/me").json()
    assert body["auth_enabled"] is False
    assert body["handle"] == "me"


def test_facets_reflect_seed(client):
    f = client.get("/v1/facets").json()
    assert "drumkits" in f["categories"]
    assert "loops" in f["categories"]
    assert "tev" in f["contributors"]
    assert "aurora" in f["packs"]
    assert "ambient" in f["tags"]


def test_search_all(client):
    res = client.get("/v1/search").json()
    assert res["total"] == 2
    assert len(res["hits"]) == 2


def test_search_by_category(client):
    res = client.get("/v1/search", params={"category": "loops"}).json()
    assert res["total"] == 1
    assert res["hits"][0]["title"] == "Dawn Loop"


def test_search_by_tag_join(client):
    res = client.get("/v1/search", params={"tag": "ambient"}).json()
    assert res["total"] == 1
    assert res["hits"][0]["instrument"] == "synth"


def test_search_bpm_range(client):
    res = client.get("/v1/search", params={"bpm_min": 120, "bpm_max": 130}).json()
    assert res["total"] == 1


def test_sample_detail_and_peaks(client):
    res = client.get("/v1/search", params={"category": "loops"}).json()
    sid = res["hits"][0]["id"]
    detail = client.get(f"/v1/sample/{sid}").json()
    assert detail["title"] == "Dawn Loop"
    assert detail["peaks"] == [[0.0, 0.1], [0.2, 0.3]]
    assert any(t["tag"] == "ambient" for t in detail["tags"])
    peaks = client.get(f"/v1/peaks/{sid}").json()
    assert peaks == [[0.0, 0.1], [0.2, 0.3]]


def test_missing_sample_404(client):
    assert client.get("/v1/sample/9999").status_code == 404


def _first_id(client) -> int:
    return client.get("/v1/search", params={"category": "loops"}).json()["hits"][0]["id"]


def test_preview_serves_audio(client):
    sid = _first_id(client)
    r = client.get(f"/v1/preview/{sid}")
    assert r.status_code == 200
    assert r.headers["content-type"].startswith("audio/")
    assert len(r.content) > 0


def test_preview_supports_range(client):
    sid = _first_id(client)
    r = client.get(f"/v1/preview/{sid}", headers={"Range": "bytes=0-9"})
    assert r.status_code == 206
    assert len(r.content) == 10
    assert r.headers["content-range"].startswith("bytes 0-9/")


def test_download_is_attachment(client):
    sid = _first_id(client)
    r = client.get(f"/v1/download/{sid}")
    assert r.status_code == 200
    assert "attachment" in r.headers.get("content-disposition", "")
    assert "loop_dawn.wav" in r.headers.get("content-disposition", "")


def test_preview_missing_404(client):
    assert client.get("/v1/preview/9999").status_code == 404
