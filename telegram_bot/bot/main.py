"""Entry point: build the Telegram application and start polling."""
from __future__ import annotations

import logging

from telegram.ext import Application, PersistenceInput, PicklePersistence

from .config import load_config
from .conversation import register_handlers
from .github_client import GitHubClient


def build_application() -> Application:
    cfg = load_config()
    # Persist only user_data (the language choice and any in-flight request
    # fields). bot_data holds live objects (Config, GitHubClient) that must be
    # rebuilt from the environment on every start, not restored from disk.
    persistence = PicklePersistence(
        filepath=cfg.persistence_file,
        store_data=PersistenceInput(
            bot_data=False, chat_data=False, user_data=True, callback_data=False
        ),
    )
    application = (
        Application.builder()
        .token(cfg.telegram_token)
        .persistence(persistence)
        .build()
    )

    application.bot_data["config"] = cfg
    application.bot_data["github"] = GitHubClient(
        owner=cfg.github_owner,
        repo=cfg.github_repo,
        token=cfg.github_token,
    )

    register_handlers(application)
    return application


def main() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(name)s %(levelname)s %(message)s",
    )

    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)

    application = build_application()
    logging.getLogger(__name__).info("HyperHLE fix bot starting (polling)…")
    application.run_polling(allowed_updates=None)


if __name__ == "__main__":
    main()
