"""Tests for the conversation handlers' validation and routing logic."""
from __future__ import annotations

from types import SimpleNamespace

from bot import conversation as c
from bot.request import FixRequest, LogFile
from conftest import (
    FakeBot,
    FakeCallbackQuery,
    FakeChat,
    FakeContext,
    FakeDocument,
    FakeMessage,
    FakePhotoSize,
    FakeUpdate,
    make_text_update,
)

REAL_HEADER = "touchHLE UNOFFICIAL 8d65eca — https://touchhle.org/"


def last_reply(update):
    return update.effective_message.replies[-1]["text"]


def stored_logs(context):
    req = context.user_data.get("request")
    return req.logs if req else []


def stored_app_name(context):
    req = context.user_data.get("request")
    return req.app_name if req else ""


def stored_app_version(context):
    req = context.user_data.get("request")
    return req.app_version if req else ""


# --- app name ---------------------------------------------------------------

async def test_got_app_name_rejects_empty(context):
    upd = make_text_update("   ​ ")
    assert await c.got_app_name(upd, context) == c.APP_NAME
    assert not stored_app_name(context)


async def test_got_app_name_rejects_more_than_six_words(context):
    upd = make_text_update("one two three four five six seven")
    assert await c.got_app_name(upd, context) == c.APP_NAME


async def test_got_app_name_accepts_and_sanitizes(context):
    upd = make_text_update("  Angry​   Birds  ")
    assert await c.got_app_name(upd, context) == c.APP_VERSION
    assert context.user_data["request"].app_name == "Angry Birds"


# --- app version ------------------------------------------------------------

async def test_got_app_version_accepts_v_prefix(context):
    upd = make_text_update("v1.0")
    assert await c.got_app_version(upd, context) == c.IPA_LINKS
    assert context.user_data["request"].app_version == "v1.0"


async def test_got_app_version_rejects_non_numeric(context):
    upd = make_text_update("the latest one")
    assert await c.got_app_version(upd, context) == c.APP_VERSION
    assert not stored_app_version(context)


# --- bug --------------------------------------------------------------------

async def test_got_bug_sanitizes_and_advances(context):
    upd = make_text_update("crash\x07 at  boot")
    assert await c.got_bug(upd, context) == c.LOGS
    assert context.user_data["request"].bug_description == "crash at boot"


async def test_got_bug_caps_length(context):
    upd = make_text_update("B" * 10_000)
    await c.got_bug(upd, context)
    assert len(context.user_data["request"].bug_description) <= c.MAX_BUG_CHARS


# --- pasted logs ------------------------------------------------------------

async def test_got_log_text_requires_header(context):
    upd = make_text_update("some log lines without a header")
    assert await c.got_log_text(upd, context) == c.LOGS
    assert not stored_logs(context)


async def test_got_log_text_accepts_real_log(context):
    upd = make_text_update(REAL_HEADER + "\nframe 1\nframe 2")
    assert await c.got_log_text(upd, context) == c.LOGS
    assert len(context.user_data["request"].logs) == 1


async def test_got_log_text_caps_pasted_length(context):
    upd = make_text_update(REAL_HEADER + "\n" + "A" * 500_000)
    await c.got_log_text(upd, context)
    stored = context.user_data["request"].logs[0].content
    assert len(stored) <= c.MAX_PASTED_LOG_CHARS


async def test_logs_count_cap(context):
    req = c._req(context)
    req.logs = [LogFile(f"l{i}.txt", REAL_HEADER) for i in range(c.MAX_LOGS)]
    upd = make_text_update(REAL_HEADER + "\nmore")
    await c.got_log_text(upd, context)
    assert len(req.logs) == c.MAX_LOGS  # unchanged


# --- log documents ----------------------------------------------------------

async def test_got_log_document_rejects_wrong_extension(context):
    doc = FakeDocument(file_name="evil.exe", file_size=10, file_id="f1")
    upd = FakeUpdate(message=FakeMessage(document=doc, bot=context.bot))
    assert await c.got_log_document(upd, context) == c.LOGS
    assert not stored_logs(context)


async def test_got_log_document_rejects_binary(context):
    context.bot.register_file("f2", bytes([0xFF] * 200))
    doc = FakeDocument(file_name="log.txt", file_size=200, file_id="f2")
    upd = FakeUpdate(message=FakeMessage(document=doc, bot=context.bot))
    assert await c.got_log_document(upd, context) == c.LOGS
    assert not stored_logs(context)


async def test_got_log_document_nameless_is_accepted_cautiously(context):
    context.bot.register_file("f3", (REAL_HEADER + "\nok").encode())
    doc = FakeDocument(file_name=None, file_size=None, file_id="f3")
    upd = FakeUpdate(message=FakeMessage(document=doc, bot=context.bot))
    assert await c.got_log_document(upd, context) == c.LOGS
    assert len(context.user_data["request"].logs) == 1


async def test_got_log_document_guards_size_when_filesize_unknown(context, monkeypatch):
    monkeypatch.setattr(c, "MAX_LOG_BYTES", 10)
    context.bot.register_file("f4", b"x" * 50)
    doc = FakeDocument(file_name=None, file_size=None, file_id="f4")
    upd = FakeUpdate(message=FakeMessage(document=doc, bot=context.bot))
    assert await c.got_log_document(upd, context) == c.LOGS
    assert not stored_logs(context)


async def test_got_log_document_requires_header(context):
    context.bot.register_file("f5", b"plain text, no header here")
    doc = FakeDocument(file_name="log.txt", file_size=30, file_id="f5")
    upd = FakeUpdate(message=FakeMessage(document=doc, bot=context.bot))
    assert await c.got_log_document(upd, context) == c.LOGS
    assert not stored_logs(context)


# --- env --------------------------------------------------------------------

async def test_got_env_splits_on_first_slash_only(context):
    upd = make_text_update("Windows 11 / NVIDIA / extra")
    await c.got_env(upd, context)
    req = context.user_data["request"]
    assert req.operating_system == "Windows 11"
    assert req.gpu == "NVIDIA / extra"


async def test_got_env_caps_lengths(context):
    upd = make_text_update("O" * 500 + " / " + "G" * 500)
    await c.got_env(upd, context)
    req = context.user_data["request"]
    assert len(req.operating_system) <= c.MAX_OS_CHARS
    assert len(req.gpu) <= c.MAX_GPU_CHARS


# --- IPA files --------------------------------------------------------------

async def test_got_ipa_file_count_cap(context):
    req = c._req(context)
    req.ipa_files = [f"a{i}.ipa" for i in range(c.MAX_IPA_FILES)]
    doc = FakeDocument(file_name="extra.ipa", file_size=1, file_id="f")
    upd = FakeUpdate(message=FakeMessage(document=doc, bot=context.bot))
    assert await c.got_ipa_file(upd, context) == c.IPA_LINKS
    assert len(req.ipa_files) == c.MAX_IPA_FILES


async def test_got_ipa_file_nameless_accepted(context):
    doc = FakeDocument(file_name=None, file_size=1, file_id="f")
    upd = FakeUpdate(message=FakeMessage(document=doc, bot=context.bot))
    assert await c.got_ipa_file(upd, context) == c.IPA_LINKS
    assert len(context.user_data["request"].ipa_files) == 1


# --- media ------------------------------------------------------------------

async def test_got_media_stores_photo_descriptor(context):
    msg = FakeMessage(photo=[FakePhotoSize(file_id="ph", file_size=500)], bot=context.bot)
    upd = FakeUpdate(message=msg)
    assert await c.got_media(upd, context) == c.MEDIA
    media = context.user_data["media"]
    assert media[0]["file_id"] == "ph" and media[0]["is_image"] is True


async def test_got_media_count_cap(context):
    context.user_data["media"] = [{"x": 1} for _ in range(c.MAX_MEDIA)]
    msg = FakeMessage(photo=[FakePhotoSize()], bot=context.bot)
    upd = FakeUpdate(message=msg)
    assert await c.got_media(upd, context) == c.MEDIA
    assert len(context.user_data["media"]) == c.MAX_MEDIA


# --- confirmation routing (#20) ---------------------------------------------

async def test_show_confirmation_routes_back_to_missing_step(context):
    req = c._req(context)
    req.app_name = "App"
    req.ipa_links = ["http://x/a.ipa"]
    req.logs = [LogFile("l.txt", REAL_HEADER)]
    # bug missing
    upd = make_text_update("/done")
    state = await c._show_confirmation(upd, context)
    assert state == c.BUG


async def test_show_confirmation_reaches_review_when_complete(context):
    req = c._req(context)
    req.app_name = "App"
    req.ipa_links = ["http://x/a.ipa"]
    req.logs = [LogFile("l.txt", REAL_HEADER)]
    req.bug_description = "boom"
    upd = make_text_update("anything")
    assert await c._show_confirmation(upd, context) == c.CONFIRM


# --- #29 already running ----------------------------------------------------

async def test_already_running_keeps_state(context):
    upd = make_text_update("/start")
    assert await c.already_running(upd, context) is None
    assert upd.message.replies


# --- #18 language guard -----------------------------------------------------

async def test_on_language_sets_valid_lang(context):
    query = FakeCallbackQuery("lang:ru", FakeMessage())
    upd = FakeUpdate(callback_query=query)
    assert await c.on_language(upd, context) == c.APP_NAME
    assert context.user_data["lang"] == "ru"


# --- #23 timeout ------------------------------------------------------------

async def test_on_timeout_notifies_and_ends(context):
    context.user_data["request"] = FixRequest()
    upd = FakeUpdate(message=FakeMessage(bot=context.bot), chat=FakeChat(777))
    assert await c.on_timeout(upd, context) == c.ConversationHandler.END
    assert any("time" in s["text"].lower() for s in context.bot.sent)
    assert "request" not in context.user_data


# --- on_confirm: markdown retry + no-token short link -----------------------

def _confirm_setup(token=None, long_link=False):
    bot = FakeBot()
    cfg = SimpleNamespace(
        forward_chat_id=None,
        issue_labels=[],
        actions_url="https://github.com/o/r/actions",
        upload_attachments=True,
        attachments_branch="telegram-bot-attachments",
        default_branch="trunk",
        forward_username="@m",
    )

    class FakeGitHub:
        can_open_issues = token is not None

        async def latest_commit(self, branch):
            return None

        async def create_issue(self, title, body, labels):
            return None

        def new_issue_link(self, title, body=""):
            base = "https://github.com/o/r/issues/new?template=app_fix_request.yml&title=T"
            return base + ("&body=" + "X" * 6000 if body else "")

        async def ensure_branch(self, *a):
            return False

        async def upload_attachment(self, *a, **k):
            return None

    ctx = FakeContext(bot=bot, bot_data={"config": cfg, "github": FakeGitHub()})
    req = FixRequest(app_name="App", bug_description="boom", reporter="@u")
    req.ipa_links = ["http://x/a.ipa"]
    req.logs = [LogFile("l.txt", REAL_HEADER)]
    ctx.user_data["request"] = req
    query = FakeCallbackQuery("submit", FakeMessage(bot=bot))
    upd = FakeUpdate(callback_query=query, chat=FakeChat(555))
    return upd, ctx, bot


async def test_on_confirm_no_token_uses_short_link_when_too_long():
    upd, ctx, bot = _confirm_setup(token=None, long_link=True)
    await c.on_confirm(upd, ctx)
    assert bot.sent, "a confirmation message should have been sent"
    final = bot.sent[-1]["text"]
    assert len(final) <= c.TELEGRAM_MAX_MESSAGE
    assert "X" * 6000 not in final  # the giant prefilled body was dropped


async def test_on_confirm_survives_markdown_failure():
    upd, ctx, bot = _confirm_setup(token=None)
    bot.fail_markdown = True  # every parse_mode send raises
    await c.on_confirm(upd, ctx)
    # The user still got their confirmation, sent as plain text on retry.
    assert bot.sent
    assert bot.sent[-1].get("parse_mode") is None
