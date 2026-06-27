"""Thin async GitHub REST client for the bot.

Two responsibilities:

* Look up the **latest commit** on a branch, so the build hash found in the
  user's log can be checked against the newest code (HyperHLE logs start with
  ``touchHLE UNOFFICIAL <shortsha> — …`` and name the branch they were built
  from).
* Open an issue from a collected fix request.

All methods degrade gracefully: if there is no token, or the network call
fails, the bot keeps working (the build check is reported as "could not
verify" and issue filing falls back to a prefilled "new issue" link).
"""
from __future__ import annotations

import base64
import urllib.parse
from dataclasses import dataclass

import httpx

API_ROOT = "https://api.github.com"


@dataclass(frozen=True)
class LatestCommit:
    sha: str
    url: str


@dataclass(frozen=True)
class CreatedIssue:
    number: int
    url: str


class GitHubClient:
    def __init__(self, owner: str, repo: str, token: str | None) -> None:
        self._owner = owner
        self._repo = repo
        self._token = token

    def _headers(self) -> dict[str, str]:
        headers = {
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "hyperhle-fix-bot",
        }
        if self._token:
            headers["Authorization"] = f"Bearer {self._token}"
        return headers

    @property
    def can_open_issues(self) -> bool:
        return self._token is not None

    async def latest_commit(self, branch: str = "trunk") -> LatestCommit | None:
        """Head commit of `branch`, or None if it can't be fetched."""
        path = (
            f"/repos/{self._owner}/{self._repo}/commits/"
            f"{urllib.parse.quote(branch)}"
        )
        try:
            async with httpx.AsyncClient(timeout=15) as client:
                resp = await client.get(API_ROOT + path, headers=self._headers())
            if resp.status_code != 200:
                return None
            data = resp.json()
            return LatestCommit(sha=data["sha"], url=data.get("html_url", ""))
        except (httpx.HTTPError, ValueError, KeyError):
            return None

    async def create_issue(
        self, title: str, body: str, labels: list[str]
    ) -> CreatedIssue | None:
        if not self._token:
            return None
        path = f"/repos/{self._owner}/{self._repo}/issues"
        payload: dict[str, object] = {"title": title, "body": body}
        if labels:
            payload["labels"] = labels
        try:
            async with httpx.AsyncClient(timeout=30) as client:
                resp = await client.post(
                    API_ROOT + path, headers=self._headers(), json=payload
                )
            if resp.status_code not in (200, 201):
                return None
            data = resp.json()
            return CreatedIssue(number=data["number"], url=data["html_url"])
        except (httpx.HTTPError, ValueError, KeyError):
            return None

    def new_issue_link(self, title: str, body: str = "") -> str:
        """A prefilled GitHub "new issue" URL (fallback when no token).

        With ``body=""`` this is a short link that just opens the template with
        the title prefilled — used when the full body would push the message
        past Telegram's size limit.
        """
        params = {"template": "app_fix_request.yml", "title": title}
        if body:
            params["body"] = body
        query = urllib.parse.urlencode(params)
        return f"https://github.com/{self._owner}/{self._repo}/issues/new?{query}"

    async def ensure_branch(self, branch: str, base_branch: str = "trunk") -> bool:
        """Make sure `branch` exists, creating it from `base_branch` if not.

        Returns True if the branch exists (or was created), False otherwise.
        Used to keep uploaded attachments off the main branch.
        """
        if not self._token:
            return False
        owner, repo = self._owner, self._repo
        ref = f"/repos/{owner}/{repo}/git/ref/heads/{urllib.parse.quote(branch)}"
        base_ref = f"/repos/{owner}/{repo}/git/ref/heads/{urllib.parse.quote(base_branch)}"
        try:
            async with httpx.AsyncClient(timeout=15) as client:
                existing = await client.get(API_ROOT + ref, headers=self._headers())
                if existing.status_code == 200:
                    return True
                base = await client.get(API_ROOT + base_ref, headers=self._headers())
                if base.status_code != 200:
                    return False
                sha = base.json()["object"]["sha"]
                created = await client.post(
                    API_ROOT + f"/repos/{owner}/{repo}/git/refs",
                    headers=self._headers(),
                    json={"ref": f"refs/heads/{branch}", "sha": sha},
                )
            return created.status_code in (200, 201)
        except (httpx.HTTPError, ValueError, KeyError):
            return False

    async def upload_attachment(
        self, path: str, data: bytes, *, branch: str, commit_message: str
    ) -> str | None:
        """Upload a binary asset via the Contents API; return its raw download
        URL, or None on failure / when there is no token."""
        if not self._token:
            return None
        # Encode each path segment but keep the slashes between folders.
        safe_path = "/".join(urllib.parse.quote(seg) for seg in path.split("/"))
        api = f"/repos/{self._owner}/{self._repo}/contents/{safe_path}"
        payload: dict[str, object] = {
            "message": commit_message,
            "content": base64.b64encode(data).decode("ascii"),
            "branch": branch,
        }
        try:
            async with httpx.AsyncClient(timeout=60) as client:
                resp = await client.put(
                    API_ROOT + api, headers=self._headers(), json=payload
                )
            if resp.status_code not in (200, 201):
                return None
            return resp.json().get("content", {}).get("download_url")
        except (httpx.HTTPError, ValueError, KeyError):
            return None
