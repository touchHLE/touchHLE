"""SQLAlchemy models and DB init for the HyperHLE app compatibility database."""
from __future__ import annotations

import os
from datetime import datetime
from pathlib import Path

from sqlalchemy import (
    Boolean,
    DateTime,
    ForeignKey,
    Integer,
    String,
    Text,
    create_engine,
    func,
)
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column, relationship, sessionmaker


# Hardcoded admin GitHub logins. Kept lowercase for case-insensitive comparison.
ADMIN_GITHUB_LOGINS: frozenset[str] = frozenset(
    {"timofeylednev", "j92580498-max", "makaherpasswordlostmedia"}
)


def _database_url() -> str:
    """Return the database URL.

    Uses ``DATABASE_URL`` if set (e.g. for Postgres), otherwise a SQLite file
    on a persistent volume in production (``/data/appdb.sqlite3``) or in the
    project root in development.
    """
    url = os.environ.get("DATABASE_URL")
    if url:
        return url
    data_dir = Path("/data") if Path("/data").is_dir() else Path(__file__).resolve().parent.parent
    data_dir.mkdir(parents=True, exist_ok=True)
    return f"sqlite:///{data_dir / 'appdb.sqlite3'}"


class Base(DeclarativeBase):
    pass


class User(Base):
    """A user authenticated via GitHub OAuth.

    We store the GitHub numeric ``id`` (immutable) plus the ``login``
    (display username; may change). ``is_admin`` is recomputed on every
    login from :data:`ADMIN_GITHUB_LOGINS`, so granting/revoking admin
    is a code change rather than an in-DB change.
    """

    __tablename__ = "users"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    github_id: Mapped[int] = mapped_column(Integer, unique=True, nullable=False, index=True)
    github_login: Mapped[str] = mapped_column(String(80), nullable=False, index=True)
    avatar_url: Mapped[str | None] = mapped_column(String(500), nullable=True)
    is_admin: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)
    created_at: Mapped[datetime] = mapped_column(
        DateTime, default=datetime.utcnow, nullable=False
    )
    last_login_at: Mapped[datetime] = mapped_column(
        DateTime, default=datetime.utcnow, nullable=False
    )

    reports: Mapped[list[Report]] = relationship(
        "Report", back_populates="reporter", foreign_keys="Report.reporter_user_id"
    )

    @property
    def display_name(self) -> str:
        return f"@{self.github_login}"


class App(Base):
    __tablename__ = "apps"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    name: Mapped[str] = mapped_column(String(200), nullable=False, unique=True, index=True)
    release_year: Mapped[int | None] = mapped_column(Integer, nullable=True)
    developer_publisher: Mapped[str | None] = mapped_column(String(200), nullable=True)
    first_reported_at: Mapped[datetime] = mapped_column(
        DateTime, default=datetime.utcnow, nullable=False
    )
    first_reported_by: Mapped[str | None] = mapped_column(String(80), nullable=True)

    reports: Mapped[list[Report]] = relationship(
        "Report", back_populates="app", cascade="all, delete-orphan"
    )


# Report.status values.
STATUS_PENDING = "pending"
STATUS_APPROVED = "approved"
STATUS_REJECTED = "rejected"
ALL_STATUSES = (STATUS_PENDING, STATUS_APPROVED, STATUS_REJECTED)


class Report(Base):
    __tablename__ = "reports"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    app_id: Mapped[int] = mapped_column(ForeignKey("apps.id"), nullable=False, index=True)

    # Version-level info (denormalised for simplicity — we group by version_number on display).
    version_number: Mapped[str] = mapped_column(String(40), nullable=False, index=True)
    display_name: Mapped[str | None] = mapped_column(String(120), nullable=True)
    bundle_identifier: Mapped[str | None] = mapped_column(String(200), nullable=True)
    minimum_ios_version: Mapped[str | None] = mapped_column(String(20), nullable=True)

    # Report-level info.
    touchhle_version: Mapped[str] = mapped_column(String(80), nullable=False)
    operating_system: Mapped[str] = mapped_column(String(120), nullable=False)
    gpu: Mapped[str | None] = mapped_column(String(120), nullable=True)
    scale_hack: Mapped[str | None] = mapped_column(String(40), nullable=True)
    # Rating 1..5, where 1=completely broken and 5=fully working.
    rating: Mapped[int] = mapped_column(Integer, nullable=False)
    remarks: Mapped[str | None] = mapped_column(Text, nullable=True)
    screenshot_url: Mapped[str | None] = mapped_column(String(500), nullable=True)
    # Filename of an uploaded screenshot stored under the uploads directory
    # (served at /uploads/{filename}). Either this or ``screenshot_url`` may
    # be set; the form lets the user pick one.
    screenshot_filename: Mapped[str | None] = mapped_column(String(255), nullable=True)

    reported_at: Mapped[datetime] = mapped_column(
        DateTime, default=datetime.utcnow, nullable=False, index=True
    )
    # Display string (e.g. "@github-login"). Always set; for legacy seed
    # data this is the only thing we have.
    reported_by: Mapped[str | None] = mapped_column(String(80), nullable=True)
    # FK to the authenticated user who submitted the report. ``None`` for
    # legacy / seed reports created before GitHub auth was enabled.
    reporter_user_id: Mapped[int | None] = mapped_column(
        ForeignKey("users.id"), nullable=True, index=True
    )

    # Moderation.
    status: Mapped[str] = mapped_column(
        String(16), nullable=False, default=STATUS_PENDING, index=True
    )
    reviewed_by_id: Mapped[int | None] = mapped_column(
        ForeignKey("users.id"), nullable=True
    )
    reviewed_at: Mapped[datetime | None] = mapped_column(DateTime, nullable=True)
    rejection_reason: Mapped[str | None] = mapped_column(Text, nullable=True)

    # Auto-triage: a scheduled Devin session can claim broken reports
    # (rating <= 3) and try to fix them. ``triage_session_id`` stores the
    # claiming session id (or any short marker); ``triage_pr_url`` is set
    # once Devin opens a PR; ``triage_notes`` is a free-text outcome from
    # the Devin session (e.g. "could not reproduce"). All three are NULL
    # for unclaimed / non-broken reports.
    triage_session_id: Mapped[str | None] = mapped_column(String(80), nullable=True, index=True)
    triage_claimed_at: Mapped[datetime | None] = mapped_column(DateTime, nullable=True)
    triage_pr_url: Mapped[str | None] = mapped_column(String(500), nullable=True)
    triage_notes: Mapped[str | None] = mapped_column(Text, nullable=True)

    app: Mapped[App] = relationship("App", back_populates="reports")
    reporter: Mapped[User | None] = relationship(
        "User", back_populates="reports", foreign_keys=[reporter_user_id]
    )
    reviewer: Mapped[User | None] = relationship("User", foreign_keys=[reviewed_by_id])


engine = create_engine(
    _database_url(),
    connect_args={"check_same_thread": False} if _database_url().startswith("sqlite") else {},
    pool_pre_ping=True,
)
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


def _sqlite_columns(conn, table: str) -> set[str]:
    return {row[1] for row in conn.exec_driver_sql(f"PRAGMA table_info({table})").all()}


def init_db() -> None:
    Base.metadata.create_all(bind=engine)
    # Lightweight migration: add columns that may be missing on databases
    # created by an older version of the app. Only handles SQLite — for
    # Postgres we'd use Alembic.
    if not engine.url.drivername.startswith("sqlite"):
        return

    with engine.begin() as conn:
        report_cols = _sqlite_columns(conn, "reports")
        if "screenshot_filename" not in report_cols:
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN screenshot_filename VARCHAR(255)"
            )
        if "reporter_user_id" not in report_cols:
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN reporter_user_id INTEGER"
            )
        if "status" not in report_cols:
            # Existing reports predate moderation; mark them as approved
            # so the site doesn't suddenly hide all the seed data.
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN status VARCHAR(16) "
                "NOT NULL DEFAULT 'approved'"
            )
        if "reviewed_by_id" not in report_cols:
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN reviewed_by_id INTEGER"
            )
        if "reviewed_at" not in report_cols:
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN reviewed_at DATETIME"
            )
        if "rejection_reason" not in report_cols:
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN rejection_reason TEXT"
            )
        if "triage_session_id" not in report_cols:
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN triage_session_id VARCHAR(80)"
            )
        if "triage_claimed_at" not in report_cols:
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN triage_claimed_at DATETIME"
            )
        if "triage_pr_url" not in report_cols:
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN triage_pr_url VARCHAR(500)"
            )
        if "triage_notes" not in report_cols:
            conn.exec_driver_sql(
                "ALTER TABLE reports ADD COLUMN triage_notes TEXT"
            )


def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()


__all__ = [
    "ADMIN_GITHUB_LOGINS",
    "App",
    "Base",
    "Report",
    "STATUS_APPROVED",
    "STATUS_PENDING",
    "STATUS_REJECTED",
    "ALL_STATUSES",
    "SessionLocal",
    "User",
    "engine",
    "func",
    "get_db",
    "init_db",
]
