"""Input sanitization, size caps and Markdown-safety helpers.

All the validation rules live here so they are defined once, reused across the
conversation handlers and renderers, and can be unit-tested without a running
bot. Two themes:

* **sizing** — every free-text field and collection has an explicit cap so a
  hostile or fat-fingered user can't overflow Telegram's 4096-char message
  limit, GitHub's 65 536-char issue-body limit, or bloat the pickled state
  file with multi-megabyte logs;
* **escaping** — user-supplied text is escaped/fenced before it is embedded in
  any Markdown context (Telegram legacy Markdown messages *and* the GitHub
  issue body), so a stray ``_`` can't make Telegram reject the message and a
  crafted ``](http://evil)`` can't inject a clickable link into a filed issue.
"""
from __future__ import annotations

import re

from telegram.helpers import escape_markdown

# --- Platform limits ---------------------------------------------------------

# Telegram rejects outgoing text messages longer than this.
TELEGRAM_MAX_MESSAGE = 4096
# GitHub rejects issue bodies longer than this.
GITHUB_MAX_ISSUE_BODY = 65_536

# --- Per-field length caps ---------------------------------------------------

MAX_APP_NAME_CHARS = 100
MAX_APP_NAME_WORDS = 6  # app/game names can't be longer than six words
MAX_APP_VERSION_CHARS = 40
MAX_BUG_CHARS = 3_000  # keeps forward_text() comfortably under the Telegram cap
MAX_OS_CHARS = 100
MAX_GPU_CHARS = 100

# --- Log caps ----------------------------------------------------------------

# One pasted log message.
MAX_PASTED_LOG_CHARS = 200_000
# All logs combined. Logs live in user_data, which main.py pickles to disk, so
# this bounds how big the state file can grow no matter how many logs are sent.
MAX_TOTAL_LOG_CHARS = 1_000_000

# --- Collection-count caps ---------------------------------------------------

MAX_IPA_FILES = 10
MAX_IPA_LINKS = 20
MAX_LOGS = 10
MAX_MEDIA = 10

# Decoded "text" with more than this fraction of U+FFFD replacement characters
# is treated as binary masquerading as text.
_BINARY_REPLACEMENT_RATIO = 0.05
_BINARY_MIN_REPLACEMENTS = 4

# Versions must be numbers (dot-separated) with an optional leading "v", e.g.
# "1.0", "v1.0.2", "12".
_VERSION_RE = re.compile(r"^v?\d+(?:\.\d+)*$", re.IGNORECASE)

# Characters stripped from every free-text field: C0 control characters (but
# not tab/newline/return) plus zero-width, BiDi-override and BOM "junk" that can
# be used to spoof, hide or reorder text.
_JUNK_RE = re.compile(
    "["
    "\x00-\x08\x0b\x0c\x0e-\x1f\x7f"  # C0 controls except \t \n \r
    "​-‏"  # zero-width space/non-joiner/joiner, LRM, RLM
    "‪-‮"  # BiDi embeddings and overrides
    "⁠⁦-⁩"  # word joiner, BiDi isolates
    "﻿"  # BOM / zero-width no-break space
    "]"
)
_HSPACE_RUN_RE = re.compile(r"[^\S\n]+")  # runs of horizontal whitespace
_BLANK_LINES_RE = re.compile(r"\n{3,}")  # 3+ consecutive newlines

# Trailing characters stripped off an extracted URL: sentence punctuation and
# Markdown delimiters that commonly cling to a pasted link.
_URL_TRAILING = ".,;:!?…)]}>\"'`*_"


def _strip_junk(s: str) -> str:
    return _JUNK_RE.sub("", s)


def sanitize_text(s: str | None, max_len: int) -> str:
    """Sanitize a single-line free-text field.

    Drops control/zero-width characters, collapses every run of whitespace
    (including newlines) to a single space, trims, and caps the length.
    """
    if not s:
        return ""
    s = _strip_junk(s)
    s = re.sub(r"\s+", " ", s).strip()
    return s[:max_len].strip()


def sanitize_multiline(s: str | None, max_len: int) -> str:
    """Sanitize a multi-line free-text field (e.g. a bug description).

    Drops control/zero-width characters and trims each line, collapses runs of
    horizontal whitespace and 3+ blank lines, but keeps single newlines so the
    user's paragraphing survives. Caps the total length.
    """
    if not s:
        return ""
    s = _strip_junk(s).replace("\r\n", "\n").replace("\r", "\n")
    lines = [_HSPACE_RUN_RE.sub(" ", line).strip() for line in s.split("\n")]
    s = _BLANK_LINES_RE.sub("\n\n", "\n".join(lines)).strip()
    return s[:max_len].strip()


def word_count(s: str) -> int:
    return len(s.split())


def is_valid_version(s: str) -> bool:
    """True if `s` is numbers, optionally led by a "v" (e.g. ``v1.0``)."""
    return bool(_VERSION_RE.match(s.strip()))


def looks_binary(text: str) -> bool:
    """Heuristic: was this "text" actually binary decoded with errors="replace"?

    A genuine UTF-8 log has no U+FFFD; a binary file decoded leniently is full
    of them.
    """
    if not text:
        return False
    replacements = text.count("�")
    if replacements < _BINARY_MIN_REPLACEMENTS:
        return False
    return replacements / len(text) > _BINARY_REPLACEMENT_RATIO


def extract_links(text: str | None, *, limit: int = MAX_IPA_LINKS) -> list[str]:
    """Pull http(s) links out of free text.

    Strips trailing sentence punctuation and Markdown delimiters that cling to
    a pasted URL, de-duplicates while preserving order, and caps the count.
    """
    out: list[str] = []
    seen: set[str] = set()
    for raw in re.findall(r"https?://\S+", text or ""):
        link = raw.rstrip(_URL_TRAILING)
        if not link or link in seen:
            continue
        seen.add(link)
        out.append(link)
        if len(out) >= limit:
            break
    return out


def md_escape(s: str | None) -> str:
    """Escape text for a Telegram legacy-Markdown message."""
    return escape_markdown(s or "", version=1)


def gh_inline_code(s: str | None) -> str:
    """Fence a value as GitHub inline code.

    Neutralises Markdown and link injection: the value can't break out of the
    code span (embedded backticks are replaced), so a crafted
    ``](http://evil)`` is rendered literally instead of becoming a link.
    """
    s = (s or "").strip()
    if not s:
        return "_n/a_"
    return "`" + s.replace("`", "ʼ") + "`"


_FILENAME_BAD_RE = re.compile(r"[^A-Za-z0-9._-]+")


def safe_filename(name: str | None, *, fallback: str = "file") -> str:
    """A filesystem/URL-safe basename for an uploaded attachment.

    Strips any directory component and replaces unsafe characters so the value
    can't be used for path traversal when building an upload path.
    """
    base = (name or "").replace("\\", "/").rsplit("/", 1)[-1]
    base = _FILENAME_BAD_RE.sub("_", base).strip("._-")
    return base[:80] or fallback


def clamp(text: str, limit: int) -> str:
    """Hard-truncate `text` to `limit` characters (final backstop)."""
    return text if len(text) <= limit else text[:limit]


def fits_telegram(text: str) -> bool:
    return len(text) <= TELEGRAM_MAX_MESSAGE
