"""The /start conversation: pick a language, collect a request, file it, forward it."""
from __future__ import annotations

import logging
import secrets

from telegram import (
    InlineKeyboardButton,
    InlineKeyboardMarkup,
    ReplyKeyboardRemove,
    Update,
)
from telegram.constants import ParseMode
from telegram.error import TelegramError
from telegram.ext import (
    Application,
    CallbackQueryHandler,
    CommandHandler,
    ContextTypes,
    ConversationHandler,
    MessageHandler,
    filters,
)

from .config import Config
from .github_client import GitHubClient
from .i18n import DEFAULT_LANG, STRINGS, t
from .request import (
    FixRequest,
    LogBuildInfo,
    LogFile,
    MediaAttachment,
    extract_build_info,
    has_log_header,
)
from .sanitize import (
    MAX_APP_NAME_CHARS,
    MAX_APP_NAME_WORDS,
    MAX_APP_VERSION_CHARS,
    MAX_BUG_CHARS,
    MAX_GPU_CHARS,
    MAX_IPA_FILES,
    MAX_LOGS,
    MAX_MEDIA,
    MAX_OS_CHARS,
    MAX_PASTED_LOG_CHARS,
    MAX_TOTAL_LOG_CHARS,
    TELEGRAM_MAX_MESSAGE,
    clamp,
    fits_telegram,
    is_valid_version,
    looks_binary,
    md_escape,
    safe_filename,
    sanitize_multiline,
    sanitize_text,
    word_count,
)

log = logging.getLogger(__name__)

# Conversation states.
LANGUAGE, APP_NAME, APP_VERSION, IPA_LINKS, BUG, LOGS, ENV, MEDIA, CONFIRM = range(9)

# Telegram bot API caps downloads at 20 MB; logs above that are rejected.
MAX_LOG_BYTES = 20 * 1024 * 1024
# Largest attachment uploaded to a GitHub issue (Telegram downloads cap at
# 20 MB; the Contents API tolerates a little more, but keep it bounded).
MAX_UPLOAD_BYTES = 20 * 1024 * 1024
# Inactivity timeout: the user has 15 minutes to answer each step.
CONVERSATION_TIMEOUT = 15 * 60

_REQUEST_KEY = "request"


def _req(context: ContextTypes.DEFAULT_TYPE) -> FixRequest:
    req = context.user_data.get(_REQUEST_KEY)
    if req is None:
        req = FixRequest()
        context.user_data[_REQUEST_KEY] = req
    return req


def _reporter(update: Update) -> str:
    user = update.effective_user
    if user is None:
        return ""
    if user.username:
        return f"@{user.username}"
    return user.full_name or str(user.id)


def _reset(context: ContextTypes.DEFAULT_TYPE) -> None:
    """Clear the in-flight request but preserve the chosen language."""
    lang = context.user_data.get("lang")
    context.user_data.clear()
    if lang:
        context.user_data["lang"] = lang


async def _reply_md(update: Update, text: str, **kwargs: object):
    """reply_text with Markdown, retried without parse_mode on failure.

    A malformed entity in interpolated user text must never cost the user the
    message (most critically the final confirmation), so we fall back to plain
    text rather than letting Telegram reject it.
    """
    message = update.effective_message
    try:
        return await message.reply_text(
            text, parse_mode=ParseMode.MARKDOWN, **kwargs
        )
    except TelegramError:
        log.warning("Markdown reply rejected; retrying as plain text")
        return await message.reply_text(text, **kwargs)


async def _send_md(
    context: ContextTypes.DEFAULT_TYPE, chat_id: int, text: str, **kwargs: object
):
    """bot.send_message with Markdown, retried without parse_mode on failure."""
    try:
        return await context.bot.send_message(
            chat_id=chat_id, text=text, parse_mode=ParseMode.MARKDOWN, **kwargs
        )
    except TelegramError:
        log.warning("Markdown send rejected; retrying as plain text")
        return await context.bot.send_message(chat_id=chat_id, text=text, **kwargs)


def _language_keyboard(prefix: str = "lang") -> InlineKeyboardMarkup:
    return InlineKeyboardMarkup(
        [
            [
                InlineKeyboardButton("🇺🇸 English", callback_data=f"{prefix}:en"),
                InlineKeyboardButton("🇷🇺 Русский", callback_data=f"{prefix}:ru"),
                InlineKeyboardButton("🇸🇦 العربية", callback_data=f"{prefix}:ar"),
            ]
        ]
    )


async def start(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    # A returning user's language survives the reset; they are only asked to
    # pick once. /language changes it later. (allow_reentry is False, so this
    # only runs when no conversation is active — a /start mid-request is caught
    # by the `already_running` fallback instead.)
    lang = context.user_data.get("lang")
    context.user_data.clear()
    if lang:
        context.user_data["lang"] = lang
    context.user_data[_REQUEST_KEY] = FixRequest(reporter=_reporter(update))
    if lang:
        await _reply_md(update, t(context, "intro"))
        return APP_NAME
    await update.message.reply_text(
        t(context, "choose_language"),
        reply_markup=_language_keyboard(),
    )
    return LANGUAGE


async def already_running(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """A /start sent while a request is already in progress (#29)."""
    await update.message.reply_text(t(context, "already_running"))
    # Returning None keeps the user in their current state.
    return None


async def on_language(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    query = update.callback_query
    await query.answer()
    # SAFETY: the CallbackQueryHandler pattern (^lang:(en|ru|ar)$) constrains
    # query.data, so the stripped value is always a valid STRINGS key. The guard
    # below is defensive only — never remove the pattern, since the value is
    # used directly as a dict key.
    lang = query.data.removeprefix("lang:")
    if lang not in STRINGS:
        lang = DEFAULT_LANG
    context.user_data["lang"] = lang
    await query.edit_message_text(t(context, "intro"), parse_mode=ParseMode.MARKDOWN)
    return APP_NAME


async def language_reprompt(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    await update.message.reply_text(
        t(context, "choose_language"),
        reply_markup=_language_keyboard(),
    )
    return LANGUAGE


async def got_app_name(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    name = sanitize_text(update.message.text, MAX_APP_NAME_CHARS)
    if not name:
        await _reply_md(update, t(context, "app_name_empty"))
        return APP_NAME
    if word_count(name) > MAX_APP_NAME_WORDS:
        await _reply_md(update, t(context, "app_name_too_many_words"))
        return APP_NAME
    _req(context).app_name = name
    await _reply_md(update, t(context, "ask_version"))
    return APP_VERSION


async def expect_app_name(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    await _reply_md(update, t(context, "expect_app_name"))
    return APP_NAME


async def got_app_version(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    version = sanitize_text(update.message.text, MAX_APP_VERSION_CHARS)
    # Versions must be numbers, optionally led by "v" (e.g. v1.0). /skip stays
    # available for users who don't know it.
    if not is_valid_version(version):
        await _reply_md(update, t(context, "app_version_invalid"))
        return APP_VERSION
    _req(context).app_version = version
    return await _ask_ipa(update, context)


async def skip_app_version(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    return await _ask_ipa(update, context)


async def _ask_ipa(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    await _reply_md(update, t(context, "ask_ipa"))
    return IPA_LINKS


async def got_ipa_links(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    req = _req(context)
    links = req.extract_links(update.message.text)
    if not links:
        await _reply_md(update, t(context, "no_link"))
        return IPA_LINKS
    req.ipa_links = links
    await _reply_md(update, t(context, "links_saved", n=len(links)))
    return BUG


async def got_ipa_file(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    """An IPA sent as a Telegram document instead of a link.

    The file is never downloaded — its message is forwarded verbatim to the
    maintainer at submit time (forwarding has no size limit), and the GitHub
    issue notes that the IPA was attached via Telegram.
    """
    req = _req(context)
    doc = update.message.document
    # A named file must look like an IPA/zip. A *nameless* document can't be
    # checked, so it is accepted cautiously (it is only forwarded, never run).
    if doc.file_name and not doc.file_name.lower().endswith((".ipa", ".zip")):
        await _reply_md(update, t(context, "ipa_not_ipa"))
        return IPA_LINKS
    if len(req.ipa_files) >= MAX_IPA_FILES:
        await update.message.reply_text(t(context, "too_many_ipa_files"))
        return IPA_LINKS
    name = sanitize_text(doc.file_name, 80) or f"app-{len(req.ipa_files) + 1}.ipa"
    req.ipa_files.append(name)
    context.user_data.setdefault("ipa_message_ids", []).append(
        update.message.message_id
    )
    await _reply_md(
        update, t(context, "ipa_file_saved", name=md_escape(name))
    )
    return IPA_LINKS


async def expect_ipa(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    await _reply_md(update, t(context, "expect_ipa"))
    return IPA_LINKS


async def ipa_done(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    req = _req(context)
    if not (req.ipa_links or req.ipa_files):
        await update.message.reply_text(t(context, "ipa_required"))
        return IPA_LINKS
    await _reply_md(update, t(context, "ask_bug"))
    return BUG


async def got_bug(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    # Cap the description so it can't overflow Telegram's 4096-char limit when
    # embedded in the maintainer forward (forward_text truncates as a backstop).
    _req(context).bug_description = sanitize_multiline(
        update.message.text, MAX_BUG_CHARS
    )
    await _reply_md(update, t(context, "ask_logs"))
    return LOGS


async def expect_bug(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    await _reply_md(update, t(context, "expect_bug"))
    return BUG


def _add_log(
    context: ContextTypes.DEFAULT_TYPE, name: str, content: str
) -> str | None:
    """Validate and store a log; return an i18n error key, or None on success.

    Enforces the count cap, the genuine-log header check, and the total stored
    size cap (logs live in user_data, which is pickled to disk).
    """
    req = _req(context)
    if len(req.logs) >= MAX_LOGS:
        return "too_many_logs"
    if not has_log_header(content):
        return "log_no_header"
    used = sum(len(lf.content) for lf in req.logs)
    remaining = MAX_TOTAL_LOG_CHARS - used
    if remaining <= 0:
        return "too_many_logs"
    if len(content) > remaining:
        content = content[:remaining] + "\n… (truncated)"
    req.logs.append(LogFile(name=name, content=content))
    return None


async def got_log_document(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    doc = update.message.document
    # A named file must be .txt/.log. A nameless document is accepted
    # cautiously here and validated by the binary/header checks below.
    if doc.file_name and not doc.file_name.lower().endswith((".txt", ".log")):
        await _reply_md(update, t(context, "log_not_txt"))
        return LOGS
    if doc.file_size and doc.file_size > MAX_LOG_BYTES:
        await update.message.reply_text(t(context, "file_too_big"))
        return LOGS
    try:
        tg_file = await context.bot.get_file(doc.file_id)
        data = bytes(await tg_file.download_as_bytearray())
    except Exception:  # noqa: BLE001 - surface a friendly message, keep collecting
        log.exception("failed to download log document")
        await update.message.reply_text(t(context, "download_failed"))
        return LOGS
    # When file_size was unknown, the pre-download size check was skipped, so
    # guard the actual downloaded length here.
    if len(data) > MAX_LOG_BYTES:
        await update.message.reply_text(t(context, "file_too_big"))
        return LOGS
    text = data.decode("utf-8", errors="replace")
    if not text.strip():
        await update.message.reply_text(t(context, "log_empty"))
        return LOGS
    # A genuine UTF-8 log has no replacement characters; a binary file decoded
    # leniently is full of them.
    if looks_binary(text):
        await _reply_md(update, t(context, "log_binary"))
        return LOGS
    name = safe_filename(doc.file_name, fallback=f"log-{len(_req(context).logs) + 1}.txt")
    err = _add_log(context, name, text)
    if err:
        await _reply_md(update, t(context, err))
        return LOGS
    await _reply_md(update, t(context, "log_added", name=md_escape(name)))
    return LOGS


async def got_log_text(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    raw = update.message.text or ""
    if not raw.strip():
        await update.message.reply_text(t(context, "log_empty"))
        return LOGS
    # Cap the pasted length before it is stored and pickled.
    content = clamp(raw, MAX_PASTED_LOG_CHARS)
    name = f"pasted-log-{len(_req(context).logs) + 1}.txt"
    err = _add_log(context, name, content)
    if err:
        await _reply_md(update, t(context, err))
        return LOGS
    await update.message.reply_text(t(context, "pasted_log_added"))
    return LOGS


async def logs_done(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    req = _req(context)
    if not req.logs:
        await update.message.reply_text(t(context, "log_required"))
        return LOGS
    await _reply_md(update, t(context, "ask_env"))
    return ENV


async def got_env(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    req = _req(context)
    text = update.message.text or ""
    # Split on the FIRST slash only, so "Windows 11 / NVIDIA / extra" keeps the
    # whole GPU instead of dropping everything after a second slash.
    os_part, sep, gpu_part = text.partition("/")
    req.operating_system = sanitize_text(os_part, MAX_OS_CHARS)
    req.gpu = sanitize_text(gpu_part, MAX_GPU_CHARS) if sep else ""
    return await _ask_media(update, context)


async def skip_env(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    return await _ask_media(update, context)


async def _ask_media(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    await _reply_md(update, t(context, "ask_media"))
    return MEDIA


def _media_descriptor(update: Update, index: int) -> dict | None:
    """Pick the file id / name / kind from a media message, or None."""
    msg = update.message
    if msg.photo:
        return {
            "file_id": msg.photo[-1].file_id,
            "name": f"screenshot-{index}.jpg",
            "is_image": True,
            "file_size": msg.photo[-1].file_size,
        }
    if msg.video:
        return {
            "file_id": msg.video.file_id,
            "name": safe_filename(msg.video.file_name, fallback=f"video-{index}.mp4"),
            "is_image": False,
            "file_size": msg.video.file_size,
        }
    if msg.animation:
        return {
            "file_id": msg.animation.file_id,
            "name": safe_filename(msg.animation.file_name, fallback=f"animation-{index}.mp4"),
            "is_image": False,
            "file_size": msg.animation.file_size,
        }
    if msg.document:
        doc = msg.document
        is_image = bool(doc.mime_type and doc.mime_type.startswith("image/"))
        return {
            "file_id": doc.file_id,
            "name": safe_filename(doc.file_name, fallback=f"attachment-{index}"),
            "is_image": is_image,
            "file_size": doc.file_size,
        }
    return None


async def got_media(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    media: list[dict] = context.user_data.setdefault("media", [])
    if len(media) >= MAX_MEDIA:
        await update.message.reply_text(t(context, "too_many_media"))
        return MEDIA
    descriptor = _media_descriptor(update, len(media) + 1)
    if descriptor is None:
        await _reply_md(update, t(context, "ask_media"))
        return MEDIA
    descriptor["message_id"] = update.message.message_id
    media.append(descriptor)
    count = len(media)
    _req(context).media_note = f"{count} screenshot(s)/video(s) attached."
    await update.message.reply_text(t(context, "media_saved", n=count))
    return MEDIA


async def media_done(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    return await _show_confirmation(update, context)


# A still-missing field routes the user back to that specific step instead of
# stranding them in MEDIA with only /cancel as an escape (#20).
_MISSING_ROUTE: dict[str, tuple[int, str]] = {
    "app_name": (APP_NAME, "expect_app_name"),
    "ipa": (IPA_LINKS, "ask_ipa"),
    "logs": (LOGS, "ask_logs"),
    "bug": (BUG, "ask_bug"),
}


async def _show_confirmation(
    update: Update, context: ContextTypes.DEFAULT_TYPE
) -> int:
    req = _req(context)
    missing = req.missing()
    if missing:
        items = ", ".join(t(context, f"missing_{key}") for key in missing)
        await update.message.reply_text(t(context, "missing", items=items))
        state, prompt = _MISSING_ROUTE[missing[0]]
        await _reply_md(update, t(context, prompt))
        return state

    app_label = req.app_name + (f" {req.app_version}" if req.app_version else "")
    summary = t(
        context,
        "review",
        app=md_escape(app_label),
        links=len(req.ipa_links) + len(req.ipa_files),
        logs=len(req.logs),
        bug=md_escape(req.bug_description[:300]),
    )
    keyboard = InlineKeyboardMarkup(
        [
            [
                InlineKeyboardButton(t(context, "btn_submit"), callback_data="submit"),
                InlineKeyboardButton(t(context, "btn_cancel"), callback_data="abort"),
            ]
        ]
    )
    await _reply_md(update, summary, reply_markup=keyboard)
    return CONFIRM


async def _upload_attachments(
    update: Update,
    context: ContextTypes.DEFAULT_TYPE,
    req: FixRequest,
    gh: GitHubClient,
    cfg: Config,
) -> None:
    """Upload logs and screenshots/video to the GitHub issue's asset branch.

    Best-effort: any failure leaves the corresponding URL empty (the issue then
    just lists the attachment by name) and never blocks submission.
    """
    if not (cfg.upload_attachments and gh.can_open_issues):
        return
    if not await gh.ensure_branch(cfg.attachments_branch, cfg.default_branch):
        return
    branch = cfg.attachments_branch
    prefix = secrets.token_hex(4)

    for i, lf in enumerate(req.logs, 1):
        data = lf.content.encode("utf-8", errors="replace")
        path = f"attachments/{prefix}/{safe_filename(lf.name, fallback=f'log-{i}.txt')}"
        url = await gh.upload_attachment(
            path, data, branch=branch, commit_message="Add log for app fix request"
        )
        if url:
            lf.url = url

    media: list[dict] = context.user_data.get("media", [])
    for i, item in enumerate(media, 1):
        name = item["name"]
        is_image = item["is_image"]
        size = item.get("file_size") or 0
        url = ""
        if not size or size <= MAX_UPLOAD_BYTES:
            try:
                tg_file = await context.bot.get_file(item["file_id"])
                data = bytes(await tg_file.download_as_bytearray())
                if len(data) <= MAX_UPLOAD_BYTES:
                    path = f"attachments/{prefix}/{safe_filename(name, fallback=f'media-{i}')}"
                    url = await gh.upload_attachment(
                        path,
                        data,
                        branch=branch,
                        commit_message="Add media for app fix request",
                    ) or ""
            except Exception:  # noqa: BLE001 - best effort per attachment
                log.warning("could not upload media %s", name)
        req.media.append(MediaAttachment(name=name, url=url, is_image=is_image))


async def on_confirm(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    query = update.callback_query
    await query.answer()
    if query.data == "abort":
        cancelled_text = t(context, "cancelled_inline")
        _reset(context)
        await query.edit_message_text(cancelled_text)
        return ConversationHandler.END

    req: FixRequest = _req(context)
    cfg: Config = context.bot_data["config"]
    gh: GitHubClient = context.bot_data["github"]

    # Make sure the request is attributed to whoever submitted it, even if the
    # reporter wasn't captured at /start (e.g. a restored conversation).
    if not req.reporter:
        req.reporter = _reporter(update)

    await query.edit_message_text(t(context, "submitting"))

    # Read the build identity out of the user's log header and compare it
    # with the latest commit on the branch the log says it was built from.
    info = LogBuildInfo()
    for log_file in req.logs:
        candidate = extract_build_info(log_file.content)
        if candidate.version:
            info = candidate
            break
    req.hyperhle_version = info.commit or info.version
    req.hyperhle_version_url = info.run_url
    latest = await gh.latest_commit(info.branch or "trunk")
    if latest is not None:
        req.latest_commit_sha = latest.sha
        if info.commit:
            req.up_to_date = latest.sha.lower().startswith(info.commit)

    # Upload logs + screenshots/video so the issue body can embed them.
    await _upload_attachments(update, context, req, gh, cfg)

    # 1) Open a GitHub issue (or fall back to a prefilled link).
    issue = await gh.create_issue(req.issue_title(), req.issue_body(), cfg.issue_labels)
    no_token = issue is None and not gh.can_open_issues
    if issue is not None:
        issue_line = t(context, "issue_line_ok", url=issue.url)
    elif no_token:
        full_url = gh.new_issue_link(req.issue_title(), req.issue_body())
        issue_line = t(context, "issue_line_no_token", url=full_url)
    else:
        issue_line = t(context, "issue_line_failed")

    # 2) Forward to the maintainer (silently — the user isn't told about it).
    await _forward_to_maintainer(update, context, req, issue)

    if req.up_to_date is True:
        build_line = t(context, "build_ok", v=req.hyperhle_version)
    elif req.up_to_date is False:
        build_line = t(
            context,
            "build_outdated",
            v=req.hyperhle_version,
            latest=req.latest_commit_sha[:7],
            actions=cfg.actions_url,
        )
    else:
        build_line = t(context, "build_unknown")

    text = t(context, "submitted") + build_line + issue_line
    # In the no-token path the prefilled link embeds the whole (URL-encoded)
    # issue body, which can blow past Telegram's limit. Fall back to a short
    # "file it here" link without the body (#2).
    if no_token and not fits_telegram(text):
        short = gh.new_issue_link(req.issue_title())
        issue_line = t(context, "issue_line_no_token_short", url=short)
        text = t(context, "submitted") + build_line + issue_line
    text = clamp(text, TELEGRAM_MAX_MESSAGE)

    await _send_md(
        context, update.effective_chat.id, text, disable_web_page_preview=True
    )
    _reset(context)
    return ConversationHandler.END


async def _forward_to_maintainer(
    update: Update,
    context: ContextTypes.DEFAULT_TYPE,
    req: FixRequest,
    issue,
) -> bool:
    cfg: Config = context.bot_data["config"]
    if cfg.forward_chat_id is None:
        return False
    try:
        text = req.forward_text()
        if issue is not None:
            text += f"\n\nIssue: {issue.url}"
        text = clamp(text, TELEGRAM_MAX_MESSAGE)
        await context.bot.send_message(
            chat_id=cfg.forward_chat_id,
            text=text,
            disable_web_page_preview=True,
        )
        # Forward attached IPA files and screenshots/videos verbatim.
        attachment_ids = context.user_data.get("ipa_message_ids", []) + [
            m["message_id"] for m in context.user_data.get("media", [])
        ]
        for msg_id in attachment_ids:
            try:
                await context.bot.forward_message(
                    chat_id=cfg.forward_chat_id,
                    from_chat_id=update.effective_chat.id,
                    message_id=msg_id,
                )
            except Exception:  # noqa: BLE001 - best effort per attachment
                log.warning("could not forward media message %s", msg_id)
        return True
    except Exception:  # noqa: BLE001
        log.exception("failed to forward to maintainer")
        return False


async def cancel(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    cancelled_text = t(context, "cancelled")
    _reset(context)
    await update.message.reply_text(
        cancelled_text, reply_markup=ReplyKeyboardRemove()
    )
    return ConversationHandler.END


async def on_timeout(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    """Fired after CONVERSATION_TIMEOUT seconds of inactivity (#23)."""
    _reset(context)
    chat = update.effective_chat if update else None
    if chat is not None:
        try:
            await context.bot.send_message(chat_id=chat.id, text=t(context, "timed_out"))
        except TelegramError:
            log.warning("could not send timeout notice")
    return ConversationHandler.END


async def language_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Standalone /language: re-show the picker, also usable mid-conversation.

    Uses the "setlang:" callback prefix so it never collides with the
    conversation's own LANGUAGE/CONFIRM callback handlers.
    """
    await update.message.reply_text(
        t(context, "choose_language"),
        reply_markup=_language_keyboard(prefix="setlang"),
    )


async def on_setlang(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    query = update.callback_query
    await query.answer()
    # SAFETY: depends on the pattern (^setlang:(en|ru|ar)$) constraining the
    # callback data — the stripped value is used directly as a STRINGS key.
    lang = query.data.removeprefix("setlang:")
    if lang not in STRINGS:
        lang = DEFAULT_LANG
    context.user_data["lang"] = lang
    await query.edit_message_text(t(context, "language_set"))


async def help_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    cfg: Config = context.bot_data["config"]
    await _reply_md(
        update,
        t(context, "help", actions=cfg.actions_url, user=cfg.forward_username),
        disable_web_page_preview=True,
    )


def build_conversation() -> ConversationHandler:
    return ConversationHandler(
        entry_points=[CommandHandler("start", start)],
        states={
            LANGUAGE: [
                CallbackQueryHandler(on_language, pattern=r"^lang:(en|ru|ar)$"),
                MessageHandler(filters.TEXT & ~filters.COMMAND, language_reprompt),
            ],
            APP_NAME: [
                MessageHandler(filters.TEXT & ~filters.COMMAND, got_app_name),
                # Catch-all for non-text (photo/sticker/etc.) so the user isn't
                # left silently stuck (#19).
                MessageHandler(~filters.TEXT & ~filters.COMMAND, expect_app_name),
            ],
            APP_VERSION: [
                CommandHandler("skip", skip_app_version),
                MessageHandler(filters.TEXT & ~filters.COMMAND, got_app_version),
            ],
            IPA_LINKS: [
                CommandHandler("done", ipa_done),
                MessageHandler(filters.Document.ALL, got_ipa_file),
                MessageHandler(filters.TEXT & ~filters.COMMAND, got_ipa_links),
                MessageHandler(
                    ~filters.TEXT & ~filters.Document.ALL & ~filters.COMMAND,
                    expect_ipa,
                ),
            ],
            BUG: [
                MessageHandler(filters.TEXT & ~filters.COMMAND, got_bug),
                MessageHandler(~filters.TEXT & ~filters.COMMAND, expect_bug),
            ],
            LOGS: [
                CommandHandler("done", logs_done),
                MessageHandler(filters.Document.ALL, got_log_document),
                MessageHandler(filters.TEXT & ~filters.COMMAND, got_log_text),
            ],
            ENV: [
                CommandHandler("skip", skip_env),
                MessageHandler(filters.TEXT & ~filters.COMMAND, got_env),
            ],
            MEDIA: [
                CommandHandler("done", media_done),
                MessageHandler(
                    (filters.PHOTO | filters.VIDEO | filters.ANIMATION | filters.Document.ALL),
                    got_media,
                ),
            ],
            CONFIRM: [CallbackQueryHandler(on_confirm, pattern=r"^(submit|abort)$")],
            ConversationHandler.TIMEOUT: [
                MessageHandler(filters.ALL, on_timeout),
                CallbackQueryHandler(on_timeout),
            ],
        },
        fallbacks=[
            CommandHandler("cancel", cancel),
            # A /start mid-request: tell the user to finish or /cancel (#29).
            CommandHandler("start", already_running),
        ],
        allow_reentry=False,
        conversation_timeout=CONVERSATION_TIMEOUT,
    )


def register_handlers(application: Application) -> None:
    application.add_handler(build_conversation())
    application.add_handler(CommandHandler("help", help_command))
    application.add_handler(CommandHandler("language", language_command))
    application.add_handler(
        CallbackQueryHandler(on_setlang, pattern=r"^setlang:(en|ru|ar)$")
    )
