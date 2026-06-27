"""The data model for a collected fix request and its renderers."""
from __future__ import annotations

import re
from dataclasses import dataclass, field

from .sanitize import (
    GITHUB_MAX_ISSUE_BODY,
    MAX_BUG_CHARS,
    clamp,
    extract_links,
    gh_inline_code,
)

# A log embedded in an issue body is truncated so a multi-megabyte paste can't
# blow past GitHub's body size limit. This is the *per-log* cap; issue_body()
# additionally enforces a total cap across all logs (GITHUB_MAX_ISSUE_BODY).
MAX_LOG_CHARS = 20_000

_URL_RE = re.compile(r"https?://\S+")

# First line of every HyperHLE / touchHLE log, e.g.
#   touchHLE UNOFFICIAL 8d65eca — https://touchhle.org/
#   HyperHLE v1.0.2 — https://touchhle.org/
_LOG_HEADER_RE = re.compile(
    r"^(?:touchHLE|HyperHLE)\s+(?:UNOFFICIAL\s+)?(?P<ver>[\w.]+)\s+[—–-]",
    re.MULTILINE | re.IGNORECASE,
)
# Second line on Actions-built binaries, e.g.
#   Built from branch "trunk" of "HyperHLE/HyperHLE" by GitHub Actions
#   workflow run https://github.com/HyperHLE/HyperHLE/actions/runs/123.
_BUILT_FROM_RE = re.compile(
    r'Built from branch "(?P<branch>[^"]+)" of "(?P<repo>[^"]+)"'
    r"\s+by GitHub Actions workflow run\s+(?P<url>https?://\S+?)\.?\s*$",
    re.MULTILINE,
)
_HEX_RE = re.compile(r"[0-9a-fA-F]{7,40}")
_BACKTICKS_RE = re.compile(r"`+")


@dataclass(frozen=True)
class LogBuildInfo:
    """Build identity extracted from a HyperHLE log header."""

    version: str = ""  # raw header version, e.g. "8d65eca" or "v1.0.2"
    commit: str = ""  # set only when the version is a commit hash
    branch: str = ""
    run_url: str = ""


def has_log_header(text: str) -> bool:
    """True if `text` begins like a genuine HyperHLE / touchHLE log.

    Used as a sanity check on uploaded/pasted logs so screenshots, random
    documents or unrelated text can't be submitted as a "log".
    """
    return bool(_LOG_HEADER_RE.search(text or ""))


def extract_build_info(text: str) -> LogBuildInfo:
    version = commit = branch = run_url = ""
    if (m := _LOG_HEADER_RE.search(text)):
        version = m.group("ver").strip()
        if _HEX_RE.fullmatch(version):
            commit = version.lower()
    if (m := _BUILT_FROM_RE.search(text)):
        branch = m.group("branch").strip()
        run_url = m.group("url").strip()
    return LogBuildInfo(version=version, commit=commit, branch=branch, run_url=run_url)


def _md_fence(content: str) -> tuple[str, str]:
    """Pick a GitHub code fence long enough that `content` can't break out."""
    longest = max((len(m.group()) for m in _BACKTICKS_RE.finditer(content)), default=0)
    ticks = "`" * max(3, longest + 1)
    return ticks, ticks


def _html_text(s: str) -> str:
    """Escape a value for use inside an HTML <summary> in the issue body."""
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


@dataclass
class LogFile:
    name: str
    content: str
    url: str = ""  # raw download URL once uploaded to the GitHub issue


@dataclass
class MediaAttachment:
    """A screenshot/video uploaded to the GitHub issue."""

    name: str
    url: str = ""  # raw download URL once uploaded
    is_image: bool = False


@dataclass
class FixRequest:
    app_name: str = ""
    app_version: str = ""
    ipa_links: list[str] = field(default_factory=list)
    ipa_files: list[str] = field(default_factory=list)  # names of attached IPAs
    bug_description: str = ""
    logs: list[LogFile] = field(default_factory=list)
    media: list[MediaAttachment] = field(default_factory=list)
    media_note: str = ""  # short internal note about attached screenshots/video
    operating_system: str = ""
    gpu: str = ""

    # Filled in at submit time from the log header + the GitHub commits API.
    hyperhle_version: str = ""  # build from the user's log
    hyperhle_version_url: str = ""  # workflow run that built it, if known
    latest_commit_sha: str = ""  # head of the branch the log was built from
    up_to_date: bool | None = None  # None = could not verify

    # Who filed it, for attribution in the forwarded message / issue footer.
    reporter: str = ""

    def extract_links(self, text: str) -> list[str]:
        return extract_links(text)

    @property
    def has_required(self) -> bool:
        return bool(
            self.app_name
            and (self.ipa_links or self.ipa_files)
            and self.bug_description
            and self.logs
        )

    def missing(self) -> list[str]:
        """Stable keys for whatever is still missing (localized by the UI)."""
        out = []
        if not self.app_name:
            out.append("app_name")
        if not (self.ipa_links or self.ipa_files):
            out.append("ipa")
        if not self.logs:
            out.append("logs")
        if not self.bug_description:
            out.append("bug")
        return out

    def _version_line(self) -> str:
        if not self.hyperhle_version:
            return "(no build hash found in the log)"
        # hyperhle_version comes from the log header regex ([\w.]+); fence it
        # anyway so it can never break the surrounding Markdown.
        ver = gh_inline_code(self.hyperhle_version)
        if self.hyperhle_version_url:
            base = f"[{ver}]({self.hyperhle_version_url})"
        else:
            base = ver
        if self.up_to_date is True:
            return base + " (✅ matches the latest commit)"
        if self.up_to_date is False:
            return (
                base
                + f" (⚠️ **outdated**, latest is {gh_inline_code(self.latest_commit_sha[:7])})"
            )
        return base + " (could not verify against the latest commit)"

    def _logs_block(self, budget: int) -> str:
        if not self.logs or budget <= 0:
            return ""
        parts: list[str] = []
        per_log = max(500, budget // max(1, len(self.logs)))
        for log in self.logs:
            content = log.content
            cap = min(MAX_LOG_CHARS, per_log)
            if len(content) > cap:
                content = content[:cap] + "\n… (truncated)"
            open_fence, close_fence = _md_fence(content)
            link = f" — [download full log]({log.url})" if log.url else ""
            parts.append(
                f"<details><summary>{_html_text(log.name)}{link}</summary>\n\n"
                f"{open_fence}\n{content}\n{close_fence}\n\n</details>"
            )
        return clamp("\n\n".join(parts), budget)

    def issue_title(self) -> str:
        name = self.app_name or "Unknown app"
        ver = f" {self.app_version}" if self.app_version else ""
        return f"[App fix] {name}{ver}"

    def _ipa_block(self) -> str:
        # IPA links are the sharpest injection vector: a crafted
        # `](http://evil)` in a "link" could otherwise become a clickable link
        # in the filed issue. Fence every link/name as inline code.
        lines = [f"- {gh_inline_code(link)}" for link in self.ipa_links]
        lines += [
            f"- {gh_inline_code(name)}: IPA file attached via Telegram "
            "(no public link)"
            for name in self.ipa_files
        ]
        return "\n".join(lines) or "_none provided_"

    def _media_block(self) -> str:
        parts: list[str] = []
        for item in self.media:
            if item.url and item.is_image:
                parts.append(f"![{_html_text(item.name)}]({item.url})")
            elif item.url:
                parts.append(f"- [{_html_text(item.name)}]({item.url})")
            else:
                parts.append(f"- {gh_inline_code(item.name)} (attached)")
        return "\n\n".join(parts)

    def issue_body(self) -> str:
        # Every user-supplied value below is fenced as inline code so malformed
        # or malicious Markdown can't break the document or inject links.
        lines = [
            f"**App / game:** {gh_inline_code(self.app_name)}",
        ]
        if self.app_version:
            lines.append(f"**App version:** {gh_inline_code(self.app_version)}")
        lines.append(
            f"**HyperHLE build (from the log):** {self._version_line()}",
        )
        if self.operating_system:
            lines.append(f"**Operating system:** {gh_inline_code(self.operating_system)}")
        if self.gpu:
            lines.append(f"**GPU:** {gh_inline_code(self.gpu)}")
        lines += [
            "",
            "### IPA link(s)",
            self._ipa_block(),
            "",
            "### Bug / crash",
        ]
        # The bug description is fenced too (it is rendered verbatim, so a
        # crafted report can't restructure the issue).
        open_fence, close_fence = _md_fence(self.bug_description)
        lines += [f"{open_fence}\n{self.bug_description}\n{close_fence}"]
        media_block = self._media_block()
        if media_block:
            lines += ["", "### Screenshots / video", media_block]

        head = "\n".join(lines)
        reporter = f" by {gh_inline_code(self.reporter)}" if self.reporter else ""
        footer = (
            "\n\n---\n"
            f"_Filed via the HyperHLE Telegram fix bot._{reporter}"
        )

        marker = "\n\n### Log file(s)\n"
        budget = GITHUB_MAX_ISSUE_BODY - len(head) - len(marker) - len(footer) - 64
        logs_block = self._logs_block(budget) or "_none provided_"
        return clamp(head + marker + logs_block + footer, GITHUB_MAX_ISSUE_BODY)

    def forward_text(self) -> str:
        """Plain-text summary for the maintainer DM (not Markdown)."""
        ipa = "\n".join(
            [f"  • {link}" for link in self.ipa_links]
            + [f"  • {name} (file, forwarded below)" for name in self.ipa_files]
        )
        log_names = ", ".join(log.name for log in self.logs) or "none"
        if self.up_to_date is True:
            build_status = " (up to date)"
        elif self.up_to_date is False:
            build_status = f" (OUTDATED, latest is {self.latest_commit_sha[:7]})"
        else:
            build_status = " (unverified)"
        out = [
            "🛠️ New app fix request",
        ]
        # Always attribute the request to whoever sent it.
        out.append(f"From: {self.reporter or 'unknown'}")
        out += [
            f"App: {self.app_name}" + (f" {self.app_version}" if self.app_version else ""),
            f"HyperHLE build (from log): {self.hyperhle_version or 'unknown'}{build_status}",
        ]
        if self.operating_system:
            out.append(f"OS: {self.operating_system}")
        if self.gpu:
            out.append(f"GPU: {self.gpu}")
        # Truncate the bug as a backstop so the DM stays under Telegram's limit
        # even if the input cap was somehow bypassed.
        bug = clamp(self.bug_description, MAX_BUG_CHARS)
        if len(bug) < len(self.bug_description):
            bug += " … (truncated)"
        out += [
            "",
            "IPA link(s):",
            ipa,
            "",
            f"Bug: {bug}",
            "",
            f"Logs attached: {log_names}",
        ]
        if self.media_note:
            out.append(f"Media: {self.media_note}")
        return "\n".join(out)
