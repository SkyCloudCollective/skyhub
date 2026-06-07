"""RanchTube — community video, embeds only (phase 1).

HTTP tests run as the dev handle 'me'. The security contract: the server parses
the URL itself, validates the provider on an allowlist, extracts a bare id, and
never reflects the raw URL into the stored/embedded value. A direct-module test
covers the parser's allowlist edges that single-user HTTP can't reach cleanly.
"""
from __future__ import annotations

import pytest

from app import tube

YT_WATCH = "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
YT_SHORT = "https://youtu.be/dQw4w9WgXcQ"
VIMEO = "https://vimeo.com/123456789"
PEERTUBE = "https://video.example.org/w/abCDef12-34"


def _post(client, url, **extra):
    return client.post("/v1/tube", json={"url": url, "title": "My set", **extra})


# ── happy paths ───────────────────────────────────────────────────────────────
def test_post_youtube_stores_provider_and_ref(client):
    r = _post(client, YT_WATCH)
    assert r.status_code == 201
    v = r.json()
    assert v["handle"] == "me"
    assert v["provider"] == "youtube"
    assert v["video_ref"] == "dQw4w9WgXcQ"
    # the embed src is rebuilt from (provider, ref) — never the raw input
    assert v["embed_url"] == "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ"
    assert v["url"] == "https://www.youtube.com/watch?v=dQw4w9WgXcQ"


def test_post_youtube_shortlink(client):
    assert _post(client, YT_SHORT).json()["video_ref"] == "dQw4w9WgXcQ"


def test_post_vimeo(client):
    v = _post(client, VIMEO).json()
    assert v["provider"] == "vimeo" and v["video_ref"] == "123456789"
    assert v["embed_url"] == "https://player.vimeo.com/video/123456789"


def test_post_peertube(client):
    v = _post(client, PEERTUBE).json()
    assert v["provider"] == "peertube"
    assert v["video_ref"] == "video.example.org/abCDef12-34"
    assert v["embed_url"] == "https://video.example.org/videos/embed/abCDef12-34"


# ── rejection (security) ──────────────────────────────────────────────────────
def test_unknown_provider_is_400(client):
    assert _post(client, "https://evil.example.com/watch?v=dQw4w9WgXcQ").status_code == 400


def test_non_https_is_400(client):
    assert _post(client, "http://www.youtube.com/watch?v=dQw4w9WgXcQ").status_code == 400


def test_javascript_scheme_is_400(client):
    assert _post(client, "javascript:alert(1)").status_code == 400


def test_missing_title_is_400(client):
    assert client.post("/v1/tube", json={"url": YT_WATCH, "title": "  "}).status_code == 400


def test_garbage_youtube_id_is_400(client):
    # right host, but the id isn't an 11-char youtube id
    assert _post(client, "https://www.youtube.com/watch?v=../etc/passwd").status_code == 400


# ── list + filters ────────────────────────────────────────────────────────────
def test_list_recent_first_and_filters(client):
    _post(client, YT_WATCH, category="tutorial")
    _post(client, VIMEO, category="set")
    lst = client.get("/v1/tube").json()
    assert len(lst) == 2
    assert lst[0]["provider"] == "vimeo"  # most recent first

    by_cat = client.get("/v1/tube?category=tutorial").json()
    assert len(by_cat) == 1 and by_cat[0]["provider"] == "youtube"

    by_handle = client.get("/v1/tube?handle=me").json()
    assert len(by_handle) == 2
    assert client.get("/v1/tube?handle=nobody").json() == []


def test_limit_is_bounded(client):
    # over-large limit is clamped, not rejected
    assert client.get("/v1/tube?limit=99999").status_code == 200


def test_get_one_and_404(client):
    vid = _post(client, YT_WATCH).json()["id"]
    assert client.get(f"/v1/tube/{vid}").json()["id"] == vid
    assert client.get("/v1/tube/999999").status_code == 404


# ── reactions + comments reuse the generic social layer ───────────────────────
def test_video_reactions_and_comments(client):
    vid = _post(client, YT_WATCH).json()["id"]
    rx = client.post(f"/v1/tube/{vid}/reactions", json={"emoji": "🔥"}).json()
    assert rx["counts"].get("🔥") == 1

    c = client.post(f"/v1/tube/{vid}/comments", json={"body": "huge"})
    assert c.status_code == 201 and c.json()["subject_type"] == "video"
    lst = client.get(f"/v1/tube/{vid}/comments").json()
    assert len(lst) == 1 and lst[0]["body"] == "huge"

    assert client.post("/v1/tube/999/comments", json={"body": "x"}).status_code == 404


def test_video_and_sample_reactions_dont_collide(client):
    # sample #1 and video #1 (same numeric id) must keep separate reaction sets
    sid = client.get("/v1/search").json()["hits"][0]["id"]
    vid = _post(client, YT_WATCH).json()["id"]
    client.post(f"/v1/sample/{sid}/reactions", json={"emoji": "🔥"})
    client.post(f"/v1/tube/{vid}/reactions", json={"emoji": "💜"})
    s_rx = client.get(f"/v1/sample/{sid}/reactions").json()["counts"]
    v_rx = client.get(f"/v1/tube/{vid}/reactions").json()["counts"]
    assert "🔥" in s_rx and "💜" not in s_rx
    assert "💜" in v_rx and "🔥" not in v_rx


# ── parser unit tests (allowlist edges) ───────────────────────────────────────
def test_parser_extracts_each_provider():
    assert tube.parse_embed(YT_WATCH) == ("youtube", "dQw4w9WgXcQ")
    assert tube.parse_embed("https://www.youtube.com/embed/dQw4w9WgXcQ") == ("youtube", "dQw4w9WgXcQ")
    assert tube.parse_embed("https://www.youtube.com/shorts/dQw4w9WgXcQ") == ("youtube", "dQw4w9WgXcQ")
    assert tube.parse_embed(VIMEO) == ("vimeo", "123456789")
    assert tube.parse_embed("https://player.vimeo.com/video/123456789") == ("vimeo", "123456789")
    assert tube.parse_embed("https://pt.example.net/videos/watch/xyz-123ABC") == (
        "peertube", "pt.example.net/xyz-123ABC",
    )


@pytest.mark.parametrize(
    "bad",
    [
        "",
        "not a url",
        "ftp://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://youtube.evil.com/watch?v=dQw4w9WgXcQ",
        "https://www.youtube.com/watch?v=tooSHORT",
        "https://vimeo.com/notanumber",
        "https://video.example.org/w/$$$",
        "https://video.example.org/random/path",
    ],
)
def test_parser_rejects_bad(bad):
    with pytest.raises(ValueError):
        tube.parse_embed(bad)


def test_rebuilt_urls_never_echo_input():
    # even with a fully attacker-styled query string, the rebuilt embed src is
    # strictly derived from the validated 11-char id
    p, ref = tube.parse_embed("https://www.youtube.com/watch?v=dQw4w9WgXcQ&onerror=alert(1)")
    assert ref == "dQw4w9WgXcQ"
    assert tube.embed_url(p, ref) == "https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ"
