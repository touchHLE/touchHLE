"""GitHub OAuth login + session-backed authentication helpers."""
from __future__ import annotations

import os
import secrets

# Optional import: deployments can drop a small ``_runtime_env.py`` next to
# this module to populate ``os.environ`` with secrets that aren't checked
# into git (OAuth client id/secret, session secret). The file is excluded
# from the repository via ``.gitignore`` and is created by the deploy
# pipeline, never by the application itself.
try:
    from . import _runtime_env  # noqa: F401  (side effects only)
except ImportError:
    pass
from datetime import datetime
from typing import Annotated
from urllib.parse import urlencode

import httpx
from fastapi import Depends, HTTPException, Request
from fastapi.responses import RedirectResponse
from sqlalchemy.orm import Session

from .db import ADMIN_GITHUB_LOGINS, User, get_db


GITHUB_AUTHORIZE_URL = "https://github.com/login/oauth/authorize"
GITHUB_TOKEN_URL = "https://github.com/login/oauth/access_token"
GITHUB_USER_URL = "https://api.github.com/user"

OAUTH_STATE_COOKIE = "hyperhle_oauth_state"
SESSION_USER_KEY = "user_id"


class OAuthConfig:
    """Read OAuth config from env vars on each access so tests / runtime
    overrides are picked up without restarting the import."""

    @property
    def client_id(self) -> str | None:
        return os.environ.get("GITHUB_OAUTH_CLIENT_ID")

    @property
    def client_secret(self) -> str | None:
        return os.environ.get("GITHUB_OAUTH_CLIENT_SECRET")

    @property
    def configured(self) -> bool:
        return bool(self.client_id and self.client_secret)


oauth_config = OAuthConfig()


def _is_admin_login(login: str | None) -> bool:
    return bool(login) and login.lower() in ADMIN_GITHUB_LOGINS


def get_current_user(
    request: Request,
    db: Annotated[Session, Depends(get_db)],
) -> User | None:
    """Return the logged-in :class:`User` or ``None`` for anonymous requests."""
    user_id = request.session.get(SESSION_USER_KEY)
    if not user_id:
        return None
    return db.query(User).filter(User.id == user_id).first()


CurrentUserDep = Annotated[User | None, Depends(get_current_user)]


def require_login(user: CurrentUserDep) -> User:
    if user is None:
        raise HTTPException(status_code=401, detail="login_required")
    return user


def require_admin(user: CurrentUserDep) -> User:
    if user is None:
        raise HTTPException(status_code=401, detail="login_required")
    if not user.is_admin:
        raise HTTPException(status_code=403, detail="admin_required")
    return user


RequireLoginDep = Annotated[User, Depends(require_login)]
RequireAdminDep = Annotated[User, Depends(require_admin)]


def _absolute_callback_url(request: Request) -> str:
    """Build the OAuth callback URL based on the incoming request.

    Honours ``X-Forwarded-Proto`` / ``-Host`` (we install
    :class:`ProxyHeadersMiddleware` so ``request.url`` is correct).
    Optionally overridable via ``OAUTH_CALLBACK_URL``.
    """
    override = os.environ.get("OAUTH_CALLBACK_URL")
    if override:
        return override
    return str(request.url_for("github_callback"))


def start_login(request: Request, next_path: str | None = None) -> RedirectResponse:
    if not oauth_config.configured:
        raise HTTPException(
            status_code=503,
            detail=(
                "GitHub login is not configured on this server. "
                "Set GITHUB_OAUTH_CLIENT_ID and GITHUB_OAUTH_CLIENT_SECRET."
            ),
        )

    state = secrets.token_urlsafe(24)
    request.session["oauth_state"] = state
    if next_path and next_path.startswith("/"):
        request.session["oauth_next"] = next_path

    qs = urlencode(
        {
            "client_id": oauth_config.client_id,
            "redirect_uri": _absolute_callback_url(request),
            "scope": "read:user",
            "state": state,
            "allow_signup": "true",
        }
    )
    return RedirectResponse(f"{GITHUB_AUTHORIZE_URL}?{qs}", status_code=303)


async def handle_callback(
    request: Request, code: str, state: str, db: Session,
) -> RedirectResponse:
    expected_state = request.session.pop("oauth_state", None)
    if not expected_state or not secrets.compare_digest(expected_state, state):
        raise HTTPException(status_code=400, detail="Invalid OAuth state.")
    if not oauth_config.configured:
        raise HTTPException(status_code=503, detail="OAuth is not configured.")

    async with httpx.AsyncClient(timeout=15.0) as client:
        token_resp = await client.post(
            GITHUB_TOKEN_URL,
            data={
                "client_id": oauth_config.client_id,
                "client_secret": oauth_config.client_secret,
                "code": code,
                "redirect_uri": _absolute_callback_url(request),
            },
            headers={"Accept": "application/json"},
        )
        if token_resp.status_code != 200:
            raise HTTPException(
                status_code=502,
                detail=f"GitHub token endpoint returned {token_resp.status_code}.",
            )
        token_data = token_resp.json()
        access_token = token_data.get("access_token")
        if not access_token:
            raise HTTPException(
                status_code=502,
                detail=f"GitHub did not return an access token: {token_data}",
            )

        user_resp = await client.get(
            GITHUB_USER_URL,
            headers={
                "Authorization": f"Bearer {access_token}",
                "Accept": "application/vnd.github+json",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        if user_resp.status_code != 200:
            raise HTTPException(
                status_code=502,
                detail=f"GitHub /user returned {user_resp.status_code}.",
            )
        gh_user = user_resp.json()

    gh_id = gh_user.get("id")
    gh_login = gh_user.get("login")
    if not gh_id or not gh_login:
        raise HTTPException(status_code=502, detail="GitHub /user response missing id/login.")

    user = db.query(User).filter(User.github_id == gh_id).first()
    is_admin = _is_admin_login(gh_login)
    now = datetime.utcnow()
    if user is None:
        user = User(
            github_id=gh_id,
            github_login=gh_login,
            avatar_url=gh_user.get("avatar_url"),
            is_admin=is_admin,
            created_at=now,
            last_login_at=now,
        )
        db.add(user)
    else:
        user.github_login = gh_login
        user.avatar_url = gh_user.get("avatar_url")
        user.is_admin = is_admin
        user.last_login_at = now
    db.commit()
    db.refresh(user)

    request.session[SESSION_USER_KEY] = user.id

    next_path = request.session.pop("oauth_next", None) or "/"
    if not next_path.startswith("/"):
        next_path = "/"
    return RedirectResponse(next_path, status_code=303)


def logout(request: Request) -> RedirectResponse:
    request.session.pop(SESSION_USER_KEY, None)
    request.session.pop("oauth_state", None)
    request.session.pop("oauth_next", None)
    return RedirectResponse("/", status_code=303)


__all__ = [
    "CurrentUserDep",
    "RequireAdminDep",
    "RequireLoginDep",
    "get_current_user",
    "handle_callback",
    "logout",
    "oauth_config",
    "require_admin",
    "require_login",
    "start_login",
]
