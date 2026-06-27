"""Runtime configuration, loaded from environment variables.

A `.env` file next to the project is loaded automatically if present, so the
bot can be run with `python -m bot.main` during development without exporting
everything by hand.
"""
from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path


def _load_dotenv() -> None:
    """Minimal .env loader (no third-party dependency).

    Only sets variables that are not already present in the environment, so
    real environment variables always win over the file.
    """
    env_path = Path(__file__).resolve().parent.parent / ".env"
    if not env_path.is_file():
        return
    for raw in env_path.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        key, value = key.strip(), value.strip().strip('"').strip("'")
        os.environ.setdefault(key, value)


@dataclass(frozen=True)
class Config:
    telegram_token: str
    forward_chat_id: int | None
    forward_username: str
    github_token: str | None
    github_owner: str
    github_repo: str
    issue_labels: list[str]
    persistence_file: Path
    # Screenshots/video and log files are uploaded to the GitHub issue. To keep
    # them off the main branch they are committed to this dedicated branch
    # (created from default_branch on first use).
    upload_attachments: bool = True
    attachments_branch: str = "telegram-bot-attachments"
    default_branch: str = "trunk"

    @property
    def repo_slug(self) -> str:
        return f"{self.github_owner}/{self.github_repo}"

    @property
    def actions_url(self) -> str:
        return f"https://github.com/{self.repo_slug}/actions"


def load_config() -> Config:
    _load_dotenv()

    token = os.environ.get("TELEGRAM_BOT_TOKEN", "").strip()
    if not token:
        raise SystemExit(
            "TELEGRAM_BOT_TOKEN is not set. Copy .env.example to .env and fill it in."
        )

    forward_raw = os.environ.get("FORWARD_CHAT_ID", "").strip()
    forward_chat_id: int | None = None
    if forward_raw:
        try:
            forward_chat_id = int(forward_raw)
        except ValueError:
            raise SystemExit("FORWARD_CHAT_ID must be a numeric Telegram chat id.")

    labels = [
        s.strip()
        for s in os.environ.get("GITHUB_ISSUE_LABELS", "app fix request").split(",")
        if s.strip()
    ]

    persistence_raw = os.environ.get("PERSISTENCE_FILE", "").strip()
    persistence_file = (
        Path(persistence_raw)
        if persistence_raw
        else Path(__file__).resolve().parent.parent / "bot_state.pickle"
    )

    upload_raw = os.environ.get("UPLOAD_ATTACHMENTS", "true").strip().lower()
    upload_attachments = upload_raw not in ("0", "false", "no", "off")

    return Config(
        telegram_token=token,
        forward_chat_id=forward_chat_id,
        forward_username=os.environ.get("FORWARD_USERNAME", "@Tog991").strip(),
        github_token=os.environ.get("GITHUB_TOKEN", "").strip() or None,
        github_owner=os.environ.get("GITHUB_OWNER", "HyperHLE").strip(),
        github_repo=os.environ.get("GITHUB_REPO", "HyperHLE").strip(),
        issue_labels=labels,
        persistence_file=persistence_file,
        upload_attachments=upload_attachments,
        attachments_branch=os.environ.get(
            "GITHUB_ATTACHMENTS_BRANCH", "telegram-bot-attachments"
        ).strip()
        or "telegram-bot-attachments",
        default_branch=os.environ.get("GITHUB_DEFAULT_BRANCH", "trunk").strip()
        or "trunk",
    )
