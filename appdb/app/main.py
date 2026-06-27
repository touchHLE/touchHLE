"""FastAPI server for the HyperHLE app compatibility database."""
from __future__ import annotations

import os
import re
import secrets as py_secrets
from collections import defaultdict
from datetime import datetime
from pathlib import Path
from typing import Annotated

from fastapi import Body, Depends, FastAPI, File, Form, Header, HTTPException, Request, UploadFile
from fastapi.responses import HTMLResponse, JSONResponse, RedirectResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from sqlalchemy import func
from sqlalchemy.orm import Session
from starlette.middleware.sessions import SessionMiddleware
from uvicorn.middleware.proxy_headers import ProxyHeadersMiddleware

from . import auth as auth_module
from .auth import (
    CurrentUserDep,
    RequireAdminDep,
    RequireLoginDep,
    handle_callback,
    logout as auth_logout,
    oauth_config,
    start_login,
)
from .db import (
    STATUS_APPROVED,
    STATUS_PENDING,
    STATUS_REJECTED,
    App,
    Report,
    User,
    get_db,
    init_db,
)
from .log_parser import parse_log
from .seed import seed

BASE_DIR = Path(__file__).resolve().parent
templates = Jinja2Templates(directory=str(BASE_DIR / "templates"))


def _uploads_dir() -> Path:
    """Directory where uploaded screenshots are stored.

    Uses ``/data/uploads`` in production (Fly volume), otherwise a local
    directory next to the project.
    """
    base = Path("/data") if Path("/data").is_dir() else BASE_DIR.parent
    p = base / "uploads"
    p.mkdir(parents=True, exist_ok=True)
    return p


RATING_LEGEND = [
    (1, "Completely broken: app crashes immediately without any user interaction."),
    (2, "Only (part of) the main menu, intro or similar is working."),
    (3, "Some of the main content of the app works, but with major issues."),
    (4, "The main content of the app works (e.g. entire game is playable) with only small issues."),
    (5, "Everything works. The app is fully usable."),
]

SCALE_HACK_CHOICES = ["Yes", "No", "Partially", "Couldn't test", "Didn't test"]

ALLOWED_IMAGE_EXTS = {".png", ".jpg", ".jpeg", ".webp", ".gif"}
MAX_SCREENSHOT_BYTES = 5 * 1024 * 1024  # 5 MB
MAX_LOG_BYTES = 5 * 1024 * 1024


def stars(rating: int | None) -> str:
    if rating is None:
        return "—"
    rating = max(1, min(5, int(rating)))
    return "⭐" * rating


def _format_dt(dt: datetime | None) -> str:
    if dt is None:
        return ""
    return dt.strftime("%Y-%m-%d %H:%M:%S")


def _iso_utc(dt: datetime | None) -> str:
    """ISO-8601 UTC with trailing Z for use in <time datetime>."""
    if dt is None:
        return ""
    return dt.strftime("%Y-%m-%dT%H:%M:%SZ")


templates.env.filters["stars"] = stars
templates.env.filters["fmt_dt"] = _format_dt
templates.env.filters["iso_utc"] = _iso_utc


app = FastAPI(title="HyperHLE app compatibility database")
# Trust X-Forwarded-* headers when running behind a TLS-terminating proxy
# (e.g. Fly). Without this, ``request.url`` reports ``http://`` and any
# absolute URLs we emit are blocked by browsers as mixed content.
app.add_middleware(ProxyHeadersMiddleware, trusted_hosts="*")

# Signed session cookies. ``SESSION_SECRET`` must be set in production;
# in development we fall back to a per-process random value (sessions
# don't survive a restart, which is fine locally).
_session_secret = os.environ.get("SESSION_SECRET") or py_secrets.token_urlsafe(48)
app.add_middleware(
    SessionMiddleware,
    secret_key=_session_secret,
    session_cookie="hyperhle_session",
    https_only=False,  # Fly terminates TLS in front of us; cookie is still flagged Secure when behind https.
    same_site="lax",
)

app.mount("/static", StaticFiles(directory=str(BASE_DIR / "static")), name="static")


@app.on_event("startup")
def _startup() -> None:
    init_db()
    seed()
    # Ensure uploads dir exists, then mount it for serving previously-uploaded
    # screenshots. Mounting happens once on startup.
    uploads = _uploads_dir()
    if not any(getattr(r, "path", None) == "/uploads" for r in app.routes):
        app.mount("/uploads", StaticFiles(directory=str(uploads)), name="uploads")


# ---------------------------------------------------------------------------
# Template globals: every template needs ``current_user`` and a few flags.
# ---------------------------------------------------------------------------


def _common_context(request: Request, db: Session) -> dict:
    user = auth_module.get_current_user(request, db)
    pending_count = 0
    if user is not None and user.is_admin:
        pending_count = db.query(Report).filter(Report.status == STATUS_PENDING).count()
    return {
        "current_user": user,
        "pending_count": pending_count,
        "oauth_configured": oauth_config.configured,
    }


# ---------------------------------------------------------------------------
# Public routes.
# ---------------------------------------------------------------------------


@app.get("/healthz")
def healthz() -> dict[str, str]:
    return {"status": "ok"}


def _approved_filter(query, user: User | None):
    """Restrict reports query: public sees only approved; the report's
    author also sees their own pending; admins see everything."""
    if user is None:
        return query.filter(Report.status == STATUS_APPROVED)
    if user.is_admin:
        return query
    return query.filter(
        (Report.status == STATUS_APPROVED) | (Report.reporter_user_id == user.id)
    )


@app.get("/", response_class=HTMLResponse)
def home(
    request: Request,
    q: str | None = None,
    db: Annotated[Session, Depends(get_db)] = None,
):
    ctx = _common_context(request, db)
    user = ctx["current_user"]

    # Apps to show: those that have at least one visible report.
    visible_reports_q = _approved_filter(db.query(Report.app_id).distinct(), user)
    visible_app_ids = {row[0] for row in visible_reports_q.all()}

    apps = (
        db.query(App)
        .filter(App.id.in_(visible_app_ids) if visible_app_ids else False)
        .order_by(App.name)
        .all()
    )

    # Aggregate best-rating / last-updated using only visible reports.
    best_rating: dict[int, int | None] = {}
    last_updated: dict[int, datetime | None] = {}
    rows = (
        _approved_filter(
            db.query(
                Report.app_id,
                func.max(Report.rating).label("best"),
                func.max(Report.reported_at).label("last_at"),
            ),
            user,
        )
        .group_by(Report.app_id)
        .all()
    )
    for app_id, best, last_at in rows:
        best_rating[app_id] = best
        last_updated[app_id] = last_at

    rating_counts: dict[int, int] = defaultdict(int)
    for app_id, best in best_rating.items():
        if best is not None:
            rating_counts[best] += 1

    if q:
        ql = q.lower().strip()
        apps = [
            a
            for a in apps
            if ql in a.name.lower()
            or (a.developer_publisher and ql in a.developer_publisher.lower())
        ]

    return templates.TemplateResponse(
        request,
        "index.html",
        {
            **ctx,
            "apps": apps,
            "best_rating": best_rating,
            "last_updated": last_updated,
            "rating_counts": rating_counts,
            "rating_legend": RATING_LEGEND,
            "q": q or "",
            "total_apps": len(apps),
        },
    )


@app.get("/apps/{app_id}", response_class=HTMLResponse)
def app_detail(
    app_id: int,
    request: Request,
    db: Annotated[Session, Depends(get_db)] = None,
):
    ctx = _common_context(request, db)
    user = ctx["current_user"]

    a = db.query(App).filter(App.id == app_id).first()
    if a is None:
        raise HTTPException(404, "App not found")

    reports = (
        _approved_filter(db.query(Report).filter(Report.app_id == app_id), user)
        .order_by(Report.reported_at.desc())
        .all()
    )

    # Hide apps with no visible reports from non-admins (404).
    if not reports and (user is None or not user.is_admin):
        raise HTTPException(404, "App not found")

    versions: dict[str, dict] = {}
    for r in reports:
        if r.status != STATUS_APPROVED:
            # Pending / rejected reports shouldn't influence the
            # aggregated "best rating" the public sees.
            continue
        v = versions.setdefault(
            r.version_number,
            {
                "version_number": r.version_number,
                "display_name": r.display_name,
                "bundle_identifier": r.bundle_identifier,
                "minimum_ios_version": r.minimum_ios_version,
                "best_rating": r.rating,
                "last_updated": r.reported_at,
                "first_reported_by": r.reported_by,
                "first_reported_at": r.reported_at,
            },
        )
        if r.rating > v["best_rating"]:
            v["best_rating"] = r.rating
        if r.reported_at > v["last_updated"]:
            v["last_updated"] = r.reported_at
        if r.reported_at < v["first_reported_at"]:
            v["first_reported_at"] = r.reported_at
            v["first_reported_by"] = r.reported_by
        if not v["display_name"] and r.display_name:
            v["display_name"] = r.display_name
        if not v["bundle_identifier"] and r.bundle_identifier:
            v["bundle_identifier"] = r.bundle_identifier
        if not v["minimum_ios_version"] and r.minimum_ios_version:
            v["minimum_ios_version"] = r.minimum_ios_version

    versions_sorted = sorted(versions.values(), key=lambda v: v["version_number"])

    return templates.TemplateResponse(
        request,
        "app_detail.html",
        {
            **ctx,
            "app": a,
            "reports": reports,
            "versions": versions_sorted,
            "rating_legend": RATING_LEGEND,
        },
    )


# ---------------------------------------------------------------------------
# Auth routes.
# ---------------------------------------------------------------------------


@app.get("/auth/github/login")
def github_login(request: Request, next: str | None = None):
    return start_login(request, next_path=next)


@app.get("/auth/github/callback", name="github_callback")
async def github_callback(
    request: Request,
    code: str | None = None,
    state: str | None = None,
    error: str | None = None,
    error_description: str | None = None,
    db: Annotated[Session, Depends(get_db)] = None,
):
    if error:
        raise HTTPException(
            status_code=400,
            detail=f"GitHub returned an error: {error} ({error_description or 'no description'}).",
        )
    if not code or not state:
        raise HTTPException(status_code=400, detail="Missing 'code' or 'state' parameter.")
    return await handle_callback(request, code, state, db)


@app.post("/auth/logout")
def logout_post(request: Request):
    return auth_logout(request)


@app.get("/auth/logout")
def logout_get(request: Request):
    return auth_logout(request)


# ---------------------------------------------------------------------------
# Submission flow.
# ---------------------------------------------------------------------------


def _empty_form_data() -> dict:
    return {
        "app_id": "",
        "new_app_name": "",
        "new_app_year": "",
        "new_app_publisher": "",
        "version_number": "",
        "display_name": "",
        "bundle_identifier": "",
        "minimum_ios_version": "",
        "touchhle_version": "",
        "operating_system": "",
        "gpu": "",
        "scale_hack": "",
        "rating": "",
        "remarks": "",
        "screenshot_url": "",
    }


def _render_submit_form(
    request: Request,
    db: Session,
    *,
    selected_app_id: int | None = None,
    form_data: dict | None = None,
    error: str | None = None,
    info: str | None = None,
    status_code: int = 200,
) -> HTMLResponse:
    ctx = _common_context(request, db)
    apps = db.query(App).order_by(App.name).all()
    selected = None
    if selected_app_id is not None:
        selected = db.query(App).filter(App.id == selected_app_id).first()
    return templates.TemplateResponse(
        request,
        "submit_report.html",
        {
            **ctx,
            "apps": apps,
            "selected_app": selected,
            "rating_legend": RATING_LEGEND,
            "scale_hack_choices": SCALE_HACK_CHOICES,
            "error": error,
            "info": info,
            "form": form_data or _empty_form_data(),
        },
        status_code=status_code,
    )


def _render_login_required(
    request: Request, db: Session, *, status_code: int = 200,
) -> HTMLResponse:
    ctx = _common_context(request, db)
    return templates.TemplateResponse(
        request,
        "login_required.html",
        {**ctx, "next_path": "/submit"},
        status_code=status_code,
    )


@app.get("/submit", response_class=HTMLResponse)
def submit_form(
    request: Request,
    app_id: int | None = None,
    db: Annotated[Session, Depends(get_db)] = None,
    user: CurrentUserDep = None,
):
    if user is None:
        return _render_login_required(request, db)
    form = _empty_form_data()
    if app_id is not None:
        form["app_id"] = str(app_id)
    return _render_submit_form(
        request, db, selected_app_id=app_id, form_data=form,
    )


@app.post("/submit/parse-log", response_class=HTMLResponse)
async def submit_parse_log(
    request: Request,
    log_file: Annotated[UploadFile, File()],
    db: Annotated[Session, Depends(get_db)] = None,
    user: CurrentUserDep = None,
):
    """Parse an uploaded HyperHLE / touchHLE log and re-render the submit form
    with the extracted fields pre-filled."""
    if user is None:
        return _render_login_required(request, db, status_code=401)
    if not log_file or not log_file.filename:
        return _render_submit_form(
            request, db, error="Please choose a log file to parse.", status_code=400,
        )
    blob = await log_file.read(MAX_LOG_BYTES + 1)
    if len(blob) > MAX_LOG_BYTES:
        return _render_submit_form(
            request, db,
            error=f"Log file too large (max {MAX_LOG_BYTES // (1024 * 1024)} MB).",
            status_code=400,
        )
    try:
        text = blob.decode("utf-8", errors="replace")
    except Exception:
        return _render_submit_form(
            request, db, error="Could not decode the log file as text.", status_code=400,
        )

    parsed = parse_log(text)

    form = _empty_form_data()
    selected_app_id: int | None = None
    if parsed.app_name:
        existing = db.query(App).filter(App.name == parsed.app_name).first()
        if existing:
            form["app_id"] = str(existing.id)
            selected_app_id = existing.id
        else:
            form["app_id"] = "__new__"
            form["new_app_name"] = parsed.app_name

    if parsed.version:
        form["version_number"] = parsed.version
    if parsed.display_name:
        form["display_name"] = parsed.display_name
    if parsed.bundle_identifier:
        form["bundle_identifier"] = parsed.bundle_identifier
    if parsed.minimum_ios_version:
        form["minimum_ios_version"] = parsed.minimum_ios_version
    if parsed.emulator_version:
        form["touchhle_version"] = parsed.emulator_version
    if parsed.operating_system:
        form["operating_system"] = parsed.operating_system
    if parsed.gpu:
        form["gpu"] = parsed.gpu
    if parsed.remarks:
        form["remarks"] = parsed.remarks

    info_bits = []
    for label, val in [
        ("app", parsed.app_name),
        ("version", parsed.version),
        ("OS", parsed.operating_system),
        ("GPU", parsed.gpu),
        ("emulator", parsed.emulator_version),
    ]:
        if val:
            info_bits.append(f"{label}={val}")
    info = (
        "Filled in from log: " + ", ".join(info_bits)
        if info_bits
        else "Could not extract any fields from the log; please fill in manually."
    )

    return _render_submit_form(
        request, db,
        selected_app_id=selected_app_id, form_data=form, info=info,
    )


def _clean(s: str | None) -> str | None:
    if s is None:
        return None
    s = s.strip()
    return s or None


_SAFE_FILENAME_RE = re.compile(r"[^A-Za-z0-9._-]+")


async def _save_uploaded_screenshot(upload: UploadFile | None) -> tuple[str | None, str | None]:
    """Save an uploaded screenshot to the uploads directory.

    Returns ``(filename, error)``. If ``upload`` is missing or empty, returns
    ``(None, None)`` (the absence of a screenshot is fine).
    """
    if upload is None or not upload.filename:
        return None, None
    ext = Path(upload.filename).suffix.lower()
    if ext not in ALLOWED_IMAGE_EXTS:
        return None, (
            f"Screenshot extension {ext or '(none)'} is not allowed; "
            f"use one of {', '.join(sorted(ALLOWED_IMAGE_EXTS))}."
        )
    blob = await upload.read(MAX_SCREENSHOT_BYTES + 1)
    if len(blob) == 0:
        return None, None
    if len(blob) > MAX_SCREENSHOT_BYTES:
        return None, f"Screenshot too large (max {MAX_SCREENSHOT_BYTES // (1024 * 1024)} MB)."
    safe_stem = _SAFE_FILENAME_RE.sub("_", Path(upload.filename).stem)[:60] or "screenshot"
    filename = f"{py_secrets.token_hex(8)}-{safe_stem}{ext}"
    out_path = _uploads_dir() / filename
    out_path.write_bytes(blob)
    return filename, None


@app.post("/submit", response_class=HTMLResponse)
async def submit_post(
    request: Request,
    app_id: Annotated[str, Form()],
    user: RequireLoginDep,
    new_app_name: Annotated[str, Form()] = "",
    new_app_year: Annotated[str, Form()] = "",
    new_app_publisher: Annotated[str, Form()] = "",
    version_number: Annotated[str, Form()] = "",
    display_name: Annotated[str, Form()] = "",
    bundle_identifier: Annotated[str, Form()] = "",
    minimum_ios_version: Annotated[str, Form()] = "",
    touchhle_version: Annotated[str, Form()] = "",
    operating_system: Annotated[str, Form()] = "",
    gpu: Annotated[str, Form()] = "",
    scale_hack: Annotated[str, Form()] = "",
    rating: Annotated[str, Form()] = "",
    remarks: Annotated[str, Form()] = "",
    screenshot_url: Annotated[str, Form()] = "",
    screenshot_file: Annotated[UploadFile | None, File()] = None,
    db: Annotated[Session, Depends(get_db)] = None,
):
    form_data = {
        "app_id": app_id,
        "new_app_name": new_app_name,
        "new_app_year": new_app_year,
        "new_app_publisher": new_app_publisher,
        "version_number": version_number,
        "display_name": display_name,
        "bundle_identifier": bundle_identifier,
        "minimum_ios_version": minimum_ios_version,
        "touchhle_version": touchhle_version,
        "operating_system": operating_system,
        "gpu": gpu,
        "scale_hack": scale_hack,
        "rating": rating,
        "remarks": remarks,
        "screenshot_url": screenshot_url,
    }

    def _err(msg: str) -> HTMLResponse:
        sel: int | None = None
        if app_id and app_id != "__new__":
            try:
                sel = int(app_id)
            except ValueError:
                sel = None
        return _render_submit_form(
            request, db,
            selected_app_id=sel, form_data=form_data, error=msg, status_code=400,
        )

    if app_id == "__new__":
        name = _clean(new_app_name)
        if not name:
            return _err("Please enter a name for the new app.")
        existing = db.query(App).filter(App.name == name).first()
        if existing:
            target_app = existing
        else:
            year_val: int | None = None
            if _clean(new_app_year):
                try:
                    year_val = int(new_app_year)
                except ValueError:
                    return _err("Release year must be a number.")
            target_app = App(
                name=name,
                release_year=year_val,
                developer_publisher=_clean(new_app_publisher),
                first_reported_by=user.display_name,
            )
            db.add(target_app)
            db.flush()
    else:
        try:
            target_app = db.query(App).filter(App.id == int(app_id)).first()
        except ValueError:
            target_app = None
        if target_app is None:
            return _err("Please choose an existing app or pick \u201cAdd a new app\u201d.")

    if not _clean(version_number):
        return _err("Version number is required.")
    if not _clean(touchhle_version):
        return _err("HyperHLE version is required.")
    if not _clean(operating_system):
        return _err("Operating system is required.")
    try:
        rating_val = int(rating)
    except (TypeError, ValueError):
        return _err("Rating is required.")
    if rating_val < 1 or rating_val > 5:
        return _err("Rating must be 1–5.")

    sh = _clean(scale_hack)
    if sh and sh not in SCALE_HACK_CHOICES:
        return _err("Invalid scale-hack value.")

    screenshot_filename, screenshot_err = await _save_uploaded_screenshot(screenshot_file)
    if screenshot_err:
        return _err(screenshot_err)

    # Auto-approve admins' submissions; everything else lands in the queue.
    if user.is_admin:
        report_status = STATUS_APPROVED
        reviewed_by_id = user.id
        reviewed_at = datetime.utcnow()
    else:
        report_status = STATUS_PENDING
        reviewed_by_id = None
        reviewed_at = None

    report = Report(
        app_id=target_app.id,
        version_number=_clean(version_number) or "",
        display_name=_clean(display_name),
        bundle_identifier=_clean(bundle_identifier),
        minimum_ios_version=_clean(minimum_ios_version),
        touchhle_version=_clean(touchhle_version) or "",
        operating_system=_clean(operating_system) or "",
        gpu=_clean(gpu),
        scale_hack=sh,
        rating=rating_val,
        remarks=_clean(remarks),
        screenshot_url=_clean(screenshot_url),
        screenshot_filename=screenshot_filename,
        reported_by=user.display_name,
        reporter_user_id=user.id,
        status=report_status,
        reviewed_by_id=reviewed_by_id,
        reviewed_at=reviewed_at,
    )
    db.add(report)
    db.commit()

    if report_status == STATUS_PENDING:
        return RedirectResponse(url="/submit/thanks", status_code=303)
    return RedirectResponse(url=f"/apps/{target_app.id}", status_code=303)


@app.get("/submit/thanks", response_class=HTMLResponse)
def submit_thanks(
    request: Request, db: Annotated[Session, Depends(get_db)] = None,
):
    ctx = _common_context(request, db)
    return templates.TemplateResponse(request, "thanks.html", ctx)


# ---------------------------------------------------------------------------
# Admin moderation queue.
# ---------------------------------------------------------------------------


@app.get("/admin", response_class=HTMLResponse)
def admin_home(
    request: Request,
    admin: RequireAdminDep,
    status: str = "pending",
    db: Annotated[Session, Depends(get_db)] = None,
):
    ctx = _common_context(request, db)
    if status not in (STATUS_PENDING, STATUS_APPROVED, STATUS_REJECTED, "all"):
        status = STATUS_PENDING
    q = db.query(Report).order_by(Report.reported_at.desc())
    if status != "all":
        q = q.filter(Report.status == status)
    reports = q.limit(200).all()
    counts = {
        STATUS_PENDING: db.query(Report).filter(Report.status == STATUS_PENDING).count(),
        STATUS_APPROVED: db.query(Report).filter(Report.status == STATUS_APPROVED).count(),
        STATUS_REJECTED: db.query(Report).filter(Report.status == STATUS_REJECTED).count(),
    }
    return templates.TemplateResponse(
        request,
        "admin.html",
        {
            **ctx,
            "reports": reports,
            "selected_status": status,
            "counts": counts,
        },
    )


@app.post("/admin/reports/{report_id}/approve")
def admin_approve(
    report_id: int,
    request: Request,
    admin: RequireAdminDep,
    db: Annotated[Session, Depends(get_db)] = None,
):
    r = db.query(Report).filter(Report.id == report_id).first()
    if r is None:
        raise HTTPException(404, "Report not found")
    r.status = STATUS_APPROVED
    r.reviewed_by_id = admin.id
    r.reviewed_at = datetime.utcnow()
    r.rejection_reason = None
    db.commit()
    return RedirectResponse(url=request.headers.get("referer") or "/admin", status_code=303)


@app.post("/admin/reports/{report_id}/reject")
def admin_reject(
    report_id: int,
    request: Request,
    admin: RequireAdminDep,
    reason: Annotated[str, Form()] = "",
    db: Annotated[Session, Depends(get_db)] = None,
):
    r = db.query(Report).filter(Report.id == report_id).first()
    if r is None:
        raise HTTPException(404, "Report not found")
    r.status = STATUS_REJECTED
    r.reviewed_by_id = admin.id
    r.reviewed_at = datetime.utcnow()
    r.rejection_reason = _clean(reason)
    db.commit()
    return RedirectResponse(url=request.headers.get("referer") or "/admin", status_code=303)


@app.post("/admin/reports/{report_id}/delete")
def admin_delete(
    report_id: int,
    request: Request,
    admin: RequireAdminDep,
    db: Annotated[Session, Depends(get_db)] = None,
):
    r = db.query(Report).filter(Report.id == report_id).first()
    if r is None:
        raise HTTPException(404, "Report not found")
    db.delete(r)
    db.commit()
    return RedirectResponse(url=request.headers.get("referer") or "/admin", status_code=303)


# ---------------------------------------------------------------------------
# Static info pages.
# ---------------------------------------------------------------------------


@app.get("/about", response_class=HTMLResponse)
def about(request: Request, db: Annotated[Session, Depends(get_db)] = None):
    return templates.TemplateResponse(
        request,
        "about.html",
        {**_common_context(request, db), "rating_legend": RATING_LEGEND},
    )


# ---------------------------------------------------------------------------
# JSON API for the scheduled "auto-triage" Devin session.
#
# A scheduled Devin session calls these endpoints once a day to find
# approved reports with rating <= 3 ("broken / has problems"), claim one,
# attempt a fix in the Rust emulator, and (best case) link a PR back via
# /api/broken-reports/{id}/triage-result.
#
# Read endpoints are public (no auth). Write endpoints require a bearer
# token from the ``APPDB_TRIAGE_TOKEN`` env var.
# ---------------------------------------------------------------------------

# Reports with rating <= TRIAGE_RATING_THRESHOLD are eligible for auto-triage.
TRIAGE_RATING_THRESHOLD = 3
# Stale claims older than this many seconds are released so a new session
# can pick the report up. 7 days = 7 * 24 * 3600.
TRIAGE_CLAIM_TTL_SECONDS = 7 * 24 * 3600


def _triage_token() -> str | None:
    return os.environ.get("APPDB_TRIAGE_TOKEN")


def _require_triage_token(authorization: str | None) -> None:
    expected = _triage_token()
    if not expected:
        raise HTTPException(
            status_code=503,
            detail="Auto-triage API is not configured on this server "
            "(APPDB_TRIAGE_TOKEN env var is unset).",
        )
    if not authorization or not authorization.lower().startswith("bearer "):
        raise HTTPException(status_code=401, detail="Missing bearer token.")
    parts = authorization.split(None, 1)
    presented = parts[1].strip() if len(parts) > 1 else ""
    if not presented:
        raise HTTPException(status_code=401, detail="Missing bearer token.")
    if not py_secrets.compare_digest(presented, expected):
        raise HTTPException(status_code=403, detail="Invalid bearer token.")


def _serialise_report(r: Report, *, full: bool) -> dict:
    base = {
        "id": r.id,
        "app_id": r.app_id,
        "app_name": r.app.name if r.app else None,
        "rating": r.rating,
        "version_number": r.version_number,
        "operating_system": r.operating_system,
        "gpu": r.gpu,
        "scale_hack": r.scale_hack,
        "touchhle_version": r.touchhle_version,
        "reported_at": _iso_utc(r.reported_at),
        "reported_by": r.reported_by,
        "url": f"/apps/{r.app_id}#report-{r.id}",
        "triage_session_id": r.triage_session_id,
        "triage_claimed_at": _iso_utc(r.triage_claimed_at),
        "triage_pr_url": r.triage_pr_url,
    }
    if full:
        base.update({
            "display_name": r.display_name,
            "bundle_identifier": r.bundle_identifier,
            "minimum_ios_version": r.minimum_ios_version,
            "remarks": r.remarks,
            "screenshot_url": r.screenshot_url,
            "screenshot_filename": r.screenshot_filename,
            "triage_notes": r.triage_notes,
        })
    return base


def _is_claim_stale(claimed_at: datetime | None) -> bool:
    if claimed_at is None:
        return True
    age = (datetime.utcnow() - claimed_at).total_seconds()
    return age > TRIAGE_CLAIM_TTL_SECONDS


@app.get("/api/broken-reports")
def api_broken_reports(
    db: Annotated[Session, Depends(get_db)] = None,
    limit: int = 25,
    include_claimed: bool = False,
) -> dict:
    """List approved reports with rating <= 3 ("broken").

    By default only returns reports that haven't been claimed by another
    Devin session (or whose claim is older than ``TRIAGE_CLAIM_TTL_SECONDS``).
    Pass ``include_claimed=true`` to see everything regardless of triage state.
    """
    limit = max(1, min(100, limit))
    q = (
        db.query(Report)
        .filter(Report.status == STATUS_APPROVED)
        .filter(Report.rating <= TRIAGE_RATING_THRESHOLD)
        .order_by(Report.reported_at.desc())
    )
    rows = q.limit(limit * 4).all()  # over-fetch so we can filter stale claims
    results = []
    for r in rows:
        if not include_claimed and r.triage_session_id and not _is_claim_stale(
            r.triage_claimed_at
        ):
            continue
        results.append(_serialise_report(r, full=False))
        if len(results) >= limit:
            break
    return {
        "count": len(results),
        "rating_threshold": TRIAGE_RATING_THRESHOLD,
        "reports": results,
    }


@app.get("/api/broken-reports/{report_id}")
def api_broken_report_detail(
    report_id: int,
    db: Annotated[Session, Depends(get_db)] = None,
) -> dict:
    r = db.query(Report).filter(Report.id == report_id).first()
    if r is None or r.status != STATUS_APPROVED:
        raise HTTPException(status_code=404, detail="Report not found.")
    return _serialise_report(r, full=True)


@app.post("/api/broken-reports/{report_id}/claim")
def api_claim_report(
    report_id: int,
    payload: Annotated[dict, Body()] = None,
    authorization: Annotated[str | None, Header()] = None,
    db: Annotated[Session, Depends(get_db)] = None,
) -> dict:
    """Claim a report for triage.

    Body: ``{"session_id": "devin-...", "force": false}``. Refuses to claim
    if the report is already claimed by another (still-fresh) session unless
    ``force`` is true.
    """
    _require_triage_token(authorization)
    payload = payload or {}
    session_id = (payload.get("session_id") or "").strip()
    force = bool(payload.get("force"))
    if not session_id:
        raise HTTPException(status_code=400, detail="session_id is required.")
    if len(session_id) > 80:
        raise HTTPException(status_code=400, detail="session_id is too long.")

    r = db.query(Report).filter(Report.id == report_id).first()
    if r is None or r.status != STATUS_APPROVED:
        raise HTTPException(status_code=404, detail="Report not found.")
    if r.rating > TRIAGE_RATING_THRESHOLD:
        raise HTTPException(
            status_code=400,
            detail=f"Report rating ({r.rating}) is above the triage threshold "
            f"({TRIAGE_RATING_THRESHOLD}).",
        )
    if (
        r.triage_session_id
        and r.triage_session_id != session_id
        and not _is_claim_stale(r.triage_claimed_at)
        and not force
    ):
        raise HTTPException(
            status_code=409,
            detail=f"Report already claimed by {r.triage_session_id} at "
            f"{_iso_utc(r.triage_claimed_at)}.",
        )

    r.triage_session_id = session_id
    r.triage_claimed_at = datetime.utcnow()
    db.commit()
    return {"ok": True, "report": _serialise_report(r, full=True)}


@app.post("/api/broken-reports/{report_id}/triage-result")
def api_triage_result(
    report_id: int,
    payload: Annotated[dict, Body()] = None,
    authorization: Annotated[str | None, Header()] = None,
    db: Annotated[Session, Depends(get_db)] = None,
) -> dict:
    """Record the outcome of a triage attempt.

    Body: ``{"session_id": "...", "pr_url": "...", "notes": "..."}``. Only
    the session that holds the claim can update the result.
    """
    _require_triage_token(authorization)
    payload = payload or {}
    session_id = (payload.get("session_id") or "").strip()
    pr_url = (payload.get("pr_url") or "").strip() or None
    notes = (payload.get("notes") or "").strip() or None
    if not session_id:
        raise HTTPException(status_code=400, detail="session_id is required.")

    r = db.query(Report).filter(Report.id == report_id).first()
    if r is None:
        raise HTTPException(status_code=404, detail="Report not found.")
    if r.triage_session_id and r.triage_session_id != session_id:
        raise HTTPException(
            status_code=403,
            detail=f"Report is claimed by a different session "
            f"({r.triage_session_id}).",
        )
    if r.triage_session_id is None:
        # Allow updating without an explicit prior claim — record the session
        # as the claimant retroactively.
        r.triage_session_id = session_id
        r.triage_claimed_at = datetime.utcnow()
    if pr_url is not None:
        r.triage_pr_url = pr_url[:500]
    if notes is not None:
        r.triage_notes = notes
    db.commit()
    return {"ok": True, "report": _serialise_report(r, full=True)}
