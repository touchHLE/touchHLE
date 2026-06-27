"""Tests for the GitHub client, using an httpx MockTransport (no network)."""
from __future__ import annotations

import base64
import json

import httpx
import pytest

from bot import github_client as gc


def _patched_client(handler):
    """Return a factory that builds an AsyncClient bound to `handler`."""
    real = httpx.AsyncClient

    def factory(*args, **kwargs):
        kwargs.pop("timeout", None)
        kwargs["transport"] = httpx.MockTransport(handler)
        return real(*args, **kwargs)

    return factory


@pytest.fixture
def patch_httpx(monkeypatch):
    def apply(handler):
        monkeypatch.setattr(httpx, "AsyncClient", _patched_client(handler))

    return apply


async def test_upload_attachment_encodes_and_returns_url(patch_httpx):
    seen = {}

    def handler(request):
        seen["body"] = json.loads(request.content)
        seen["path"] = request.url.path
        return httpx.Response(
            201, json={"content": {"download_url": "https://raw/x/file.png"}}
        )

    patch_httpx(handler)
    client = gc.GitHubClient("o", "r", "tok")
    url = await client.upload_attachment(
        "attachments/ab/file.png", b"\x89PNG", branch="b", commit_message="m"
    )
    assert url == "https://raw/x/file.png"
    assert base64.b64decode(seen["body"]["content"]) == b"\x89PNG"
    assert seen["body"]["branch"] == "b"


async def test_upload_attachment_no_token_returns_none():
    client = gc.GitHubClient("o", "r", None)
    assert await client.upload_attachment("p", b"x", branch="b", commit_message="m") is None


async def test_upload_attachment_handles_error_status(patch_httpx):
    patch_httpx(lambda req: httpx.Response(422, json={}))
    client = gc.GitHubClient("o", "r", "tok")
    assert await client.upload_attachment("p", b"x", branch="b", commit_message="m") is None


async def test_ensure_branch_returns_true_when_exists(patch_httpx):
    def handler(request):
        if request.url.path.endswith("/git/ref/heads/branch"):
            return httpx.Response(200, json={"object": {"sha": "abc"}})
        return httpx.Response(404)

    patch_httpx(handler)
    client = gc.GitHubClient("o", "r", "tok")
    assert await client.ensure_branch("branch", "trunk") is True


async def test_ensure_branch_creates_when_missing(patch_httpx):
    calls = []

    def handler(request):
        calls.append((request.method, request.url.path))
        if "ref/heads/missing" in request.url.path:
            return httpx.Response(404, json={})
        if "ref/heads/trunk" in request.url.path:
            return httpx.Response(200, json={"object": {"sha": "basesha"}})
        if request.method == "POST" and request.url.path.endswith("/git/refs"):
            return httpx.Response(201, json={})
        return httpx.Response(500)

    patch_httpx(handler)
    client = gc.GitHubClient("o", "r", "tok")
    assert await client.ensure_branch("missing", "trunk") is True
    assert any(m == "POST" and p.endswith("/git/refs") for m, p in calls)


async def test_create_issue(patch_httpx):
    patch_httpx(
        lambda req: httpx.Response(
            201, json={"number": 5, "html_url": "https://github.com/o/r/issues/5"}
        )
    )
    client = gc.GitHubClient("o", "r", "tok")
    issue = await client.create_issue("t", "b", ["lbl"])
    assert issue.number == 5
    assert issue.url.endswith("/issues/5")


def test_new_issue_link_short_has_no_body():
    client = gc.GitHubClient("o", "r", None)
    short = client.new_issue_link("My Title")
    assert "body=" not in short
    assert "title=My+Title" in short
    full = client.new_issue_link("My Title", "the body")
    assert "body=the+body" in full
