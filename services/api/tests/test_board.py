"""Transparency layer — roadmap, feature requests + votes, feedback.

HTTP tests run as the dev handle 'me'; the multi-member vote ordering is covered
by a direct-module test (single-user HTTP can't cast votes as other handles)."""
from __future__ import annotations

from app import board


def test_roadmap_empty_by_default(client):
    assert client.get("/v1/roadmap").json() == []


def test_feature_request_lifecycle(client):
    r = client.post(
        "/v1/feature-requests", json={"title": "Dark mode for the galaxy", "body": "please"}
    )
    assert r.status_code == 201
    fr = r.json()
    assert fr["handle"] == "me"
    assert fr["votes"] == 1 and fr["voted"] is True  # proposing implies a vote
    rid = fr["id"]

    lst = client.get("/v1/feature-requests").json()
    assert any(x["id"] == rid and x["votes"] == 1 and x["voted"] for x in lst)

    v = client.delete(f"/v1/feature-requests/{rid}/vote").json()
    assert v["votes"] == 0 and v["voted"] is False
    v = client.post(f"/v1/feature-requests/{rid}/vote").json()
    assert v["votes"] == 1 and v["voted"] is True


def test_feature_request_requires_title(client):
    assert client.post("/v1/feature-requests", json={"title": "   "}).status_code == 400


def test_vote_unknown_request_404(client):
    assert client.post("/v1/feature-requests/9999/vote").status_code == 404
    assert client.delete("/v1/feature-requests/9999/vote").status_code == 404


def test_feedback_submit(client):
    r = client.post("/v1/feedback", json={"message": "love the studio", "context": "/studio"})
    assert r.status_code == 201 and r.json()["ok"] is True


def test_feedback_requires_message(client):
    assert client.post("/v1/feedback", json={"message": ""}).status_code == 400


def test_feature_requests_ordered_by_votes(wconn):
    a = board.create_feature_request(wconn, "me", "A", None)["id"]
    b = board.create_feature_request(wconn, "me", "B", None)["id"]
    # B gets two extra votes from other members -> 3 total vs A's 1
    board.vote(wconn, b, "alice", True)
    board.vote(wconn, b, "bob", True)
    lst = board.list_feature_requests(wconn, "me")
    assert lst[0]["id"] == b and lst[0]["votes"] == 3
    assert lst[-1]["id"] == a and lst[-1]["votes"] == 1
