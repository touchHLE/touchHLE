"""Tests for the FixRequest renderers and log-header detection."""
from __future__ import annotations

from bot.request import (
    FixRequest,
    LogFile,
    MediaAttachment,
    has_log_header,
)
from bot.sanitize import GITHUB_MAX_ISSUE_BODY, TELEGRAM_MAX_MESSAGE

REAL_HEADER = "touchHLE UNOFFICIAL 8d65eca — https://touchhle.org/"


def _basic_request(**overrides):
    req = FixRequest(app_name="Test App", bug_description="it crashes")
    req.ipa_links = ["http://example.com/app.ipa"]
    req.logs = [LogFile("log.txt", REAL_HEADER + "\nmore")]
    for k, v in overrides.items():
        setattr(req, k, v)
    return req


def test_has_log_header():
    assert has_log_header(REAL_HEADER)
    assert has_log_header("HyperHLE v1.0.2 — https://touchhle.org/")
    assert not has_log_header("just some text without a header")
    assert not has_log_header("")


def test_issue_body_fences_all_user_values():
    req = _basic_request(
        app_name="Evil*App_[x]",
        app_version="v1.0",
        operating_system="Win/11",
        gpu="GTX 1660",
        reporter="@ha_x",
    )
    body = req.issue_body()
    assert "`Evil*App_[x]`" in body
    assert "`v1.0`" in body
    assert "`Win/11`" in body
    assert "`@ha_x`" in body


def test_issue_body_neutralizes_ipa_link_injection():
    req = _basic_request()
    req.ipa_links = ["http://ok/a.ipa](http://evil.example)"]
    body = req.issue_body()
    # The crafted link is rendered as inline code, not a Markdown link.
    assert "`http://ok/a.ipa](http://evil.example)`" in body
    assert "](http://evil.example)" not in body.replace(
        "`http://ok/a.ipa](http://evil.example)`", ""
    )


def test_issue_body_fences_bug_with_dynamic_fence():
    req = _basic_request(bug_description="```\nnot a real fence\n```")
    body = req.issue_body()
    # A 3-backtick run in the bug forces a 4-backtick fence around it.
    assert "````" in body


def test_issue_body_respects_total_cap():
    req = _basic_request()
    req.logs = [LogFile(f"log{i}.txt", REAL_HEADER + "\n" + "A" * 500_000) for i in range(10)]
    body = req.issue_body()
    assert len(body) <= GITHUB_MAX_ISSUE_BODY


def test_issue_body_embeds_uploaded_media_and_logs():
    req = _basic_request()
    req.logs[0].url = "https://raw.githubusercontent.com/o/r/b/log.txt"
    req.media = [
        MediaAttachment("shot.png", "https://raw/x/shot.png", is_image=True),
        MediaAttachment("clip.mp4", "https://raw/x/clip.mp4", is_image=False),
    ]
    body = req.issue_body()
    assert "![shot.png](https://raw/x/shot.png)" in body
    assert "[clip.mp4](https://raw/x/clip.mp4)" in body
    assert "[download full log](https://raw.githubusercontent.com/o/r/b/log.txt)" in body


def test_forward_text_includes_reporter_username():
    req = _basic_request(reporter="@someone")
    text = req.forward_text()
    assert "@someone" in text
    assert "From: @someone" in text


def test_forward_text_truncates_long_bug():
    req = _basic_request(bug_description="B" * 10_000, reporter="@u")
    text = req.forward_text()
    assert len(text) <= TELEGRAM_MAX_MESSAGE
    assert "truncated" in text


def test_extract_links_method_dedupes():
    req = FixRequest()
    assert req.extract_links("a http://x/y.ipa http://x/y.ipa b") == ["http://x/y.ipa"]
