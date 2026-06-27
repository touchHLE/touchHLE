"""Parser for HyperHLE / touchHLE log output.

Given a raw log file uploaded by a user, extract the fields that match the
columns on the submit form (app name, version, bundle id, minimum iOS, GPU,
emulator version, OS guess) so we can pre-fill the form for them.
"""
from __future__ import annotations

import re
from dataclasses import dataclass


@dataclass(slots=True)
class ParsedLog:
    app_name: str | None = None
    display_name: str | None = None
    version: str | None = None
    bundle_identifier: str | None = None
    minimum_ios_version: str | None = None
    emulator_version: str | None = None  # e.g. "v0.2.3" or "UNOFFICIAL 9424a29c"
    operating_system: str | None = None  # best-effort guess
    gpu: str | None = None
    remarks: str | None = None  # auto-extracted error/panic line if present


# Header line is the first line of every HyperHLE / touchHLE log.
#   touchHLE UNOFFICIAL 9424a29c — https://touchhle.org/
#   touchHLE v0.2.3 — https://touchhle.org/
#   HyperHLE v0.1.0 — ...
_HEADER_RE = re.compile(
    r"^(?:touchHLE|HyperHLE)\s+(?P<ver>[^\n—–-]+?)\s+[—–-]",
    re.MULTILINE | re.IGNORECASE,
)

# "App bundle info:" block fields.
_DISPLAY_NAME_RE = re.compile(r"^-\s*Display name:\s*(?P<v>.+?)\s*$", re.MULTILINE)
_VERSION_RE = re.compile(r"^-\s*Version:\s*(?P<v>.+?)\s*$", re.MULTILINE)
_IDENT_RE = re.compile(r"^-\s*Identifier:\s*(?P<v>.+?)\s*$", re.MULTILINE)
_MIN_OS_RE = re.compile(r"^-\s*Minimum OS version:\s*(?P<v>.+?)\s*$", re.MULTILINE)

# "Driver info:" line, e.g.
#   touchHLE::window: Driver info: OpenGL ES-CM 1.1 v1.r32p1-... / ARM / Mali-G57 MC2
_DRIVER_INFO_RE = re.compile(
    r"(?:touchHLE|HyperHLE)::window:\s*Driver info:\s*(?P<v>.+?)\s*$",
    re.MULTILINE | re.IGNORECASE,
)

# Android base path detection.
_ANDROID_PATH_RE = re.compile(r"/Android/(?:data|obb)/", re.IGNORECASE)
# A few hints for desktop OSes (from log lines that mention Wine etc.).
_WINDOWS_HINT_RE = re.compile(r"\b(Windows|win32|win64)\b", re.IGNORECASE)
_MACOS_HINT_RE = re.compile(r"\b(Apple M[0-9]|macOS|Sequoia|Sonoma|Ventura)\b", re.IGNORECASE)
_LINUX_HINT_RE = re.compile(r"\bLinux\b", re.IGNORECASE)

# Crashes / panics.
_PANIC_RE = re.compile(
    r"(?:thread '[^']+' panicked at[^\n]+|panicked at\s+[^\n]+|"
    r"^.*?Error:[^\n]+|^.*?\bUnimplemented\b[^\n]+|^.*?\bunhandled\b[^\n]+)",
    re.MULTILINE | re.IGNORECASE,
)


def _gpu_from_driver_info(driver: str) -> str | None:
    """Extract the human GPU name from the "Driver info" string.

    Format: ``OpenGL ES-CM 1.1 v1.r32p1-... / ARM / Mali-G57 MC2``
    We want ``ARM Mali-G57 MC2``.
    """
    parts = [p.strip() for p in driver.split("/") if p.strip()]
    if len(parts) >= 2:
        # Last 2 parts are vendor and renderer in standard touchHLE format.
        return " ".join(parts[-2:])
    return driver.strip() or None


def parse_log(text: str) -> ParsedLog:
    out = ParsedLog()

    if (m := _HEADER_RE.search(text)):
        out.emulator_version = m.group("ver").strip()

    if (m := _DISPLAY_NAME_RE.search(text)):
        out.display_name = m.group("v").strip()
        out.app_name = out.display_name
    if (m := _VERSION_RE.search(text)):
        out.version = m.group("v").strip()
    if (m := _IDENT_RE.search(text)):
        out.bundle_identifier = m.group("v").strip()
    if (m := _MIN_OS_RE.search(text)):
        out.minimum_ios_version = m.group("v").strip()

    if (m := _DRIVER_INFO_RE.search(text)):
        out.gpu = _gpu_from_driver_info(m.group("v"))

    # OS guess — order matters (Android base-path is very specific).
    if _ANDROID_PATH_RE.search(text):
        out.operating_system = "Android"
    elif _MACOS_HINT_RE.search(text):
        out.operating_system = "macOS"
    elif _WINDOWS_HINT_RE.search(text):
        out.operating_system = "Windows"
    elif _LINUX_HINT_RE.search(text):
        out.operating_system = "Linux"

    # Find first interesting error/panic line for remarks.
    panic = _PANIC_RE.search(text)
    if panic:
        line = panic.group(0).strip()
        if len(line) > 400:
            line = line[:400] + "…"
        out.remarks = f"Auto-extracted from log: {line}"

    return out


__all__ = ["ParsedLog", "parse_log"]
