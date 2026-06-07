"""Follows, the follow-feed, and the notifications inbox. HTTP tests run as the
dev handle 'me'; notification *generation* (recipient != me) is covered by
direct-module tests, since single-user HTTP can't act as another member."""
from __future__ import annotations

from app import community, notify, social


def _tev_sample(conn) -> int:
    return conn.execute("SELECT id FROM samples WHERE contributor='tev'").fetchone()[0]


# ── follows over HTTP ────────────────────────────────────────────────────────
def test_follow_self_is_400(client):
    assert client.post("/v1/follow/me").status_code == 400


def test_follow_lifecycle_and_profile(client):
    r = client.post("/v1/follow/tev").json()
    assert r["followers"] == 1 and r["you_follow"] is True

    prof = client.get("/v1/profile/tev").json()
    assert prof["you_follow"] is True and prof["followers"] == 1
    assert "tev" in client.get("/v1/me/following").json()

    u = client.delete("/v1/follow/tev").json()
    assert u["followers"] == 0 and u["you_follow"] is False
    assert "tev" not in client.get("/v1/me/following").json()


def test_feed_shows_followed_contributions(client):
    assert client.get("/v1/feed").json() == []  # following nobody yet
    client.post("/v1/follow/tev")
    feed = client.get("/v1/feed").json()
    assert feed and all(s["contributor"] == "tev" for s in feed)


def test_notifications_inbox_starts_empty(client):
    n = client.get("/v1/notifications").json()
    assert n["unread"] == 0 and n["items"] == []


# ── notification generation (direct module: needs a second actor) ────────────-
def test_comment_notifies_contributor_not_self(wconn):
    sid = _tev_sample(wconn)
    social.add_comment(wconn, sid, "alice", "love this kick")
    inbox = notify.list_for(wconn, "tev")
    assert inbox["unread"] == 1
    assert inbox["items"][0]["kind"] == "comment" and inbox["items"][0]["actor"] == "alice"

    social.add_comment(wconn, sid, "tev", "thanks")  # self-comment: no notification
    assert notify.list_for(wconn, "tev")["unread"] == 1

    notify.mark_all_read(wconn, "tev")
    assert notify.list_for(wconn, "tev")["unread"] == 0


def test_reaction_and_follow_notify(wconn):
    sid = _tev_sample(wconn)
    social.toggle_reaction(wconn, sid, "alice", "🔥", True)
    community.set_follow(wconn, "bob", "tev", True)
    inbox = notify.list_for(wconn, "tev")
    kinds = {i["kind"] for i in inbox["items"]}
    assert inbox["unread"] == 2 and {"reaction", "follow"} <= kinds
