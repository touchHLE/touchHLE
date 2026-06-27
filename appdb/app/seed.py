"""Seed the database with a small set of example apps and reports.

This is purely demo data so a fresh deployment isn't empty. It mirrors a few
well-known apps from https://appdb.touchhle.org/ but the reports are
illustrative — they are not copied verbatim.
"""
from __future__ import annotations

from datetime import datetime, timedelta

from .db import STATUS_APPROVED, App, Report, SessionLocal, init_db


def _now_minus(days: int) -> datetime:
    return datetime.utcnow() - timedelta(days=days)


SEED_APPS: list[dict] = [
    {
        "app": dict(
            name="Super Monkey Ball",
            release_year=2008,
            developer_publisher="Other Ocean Interactive / SEGA",
            first_reported_by="@hikari-no-yume",
            first_reported_at=_now_minus(420),
        ),
        "reports": [
            dict(
                version_number="1.3",
                display_name="Monkey Ball",
                bundle_identifier="com.ooi.supermonkeyball",
                minimum_ios_version="2.0",
                touchhle_version="v0.2.3",
                operating_system="Windows 11 23H2",
                gpu="RTX 2060",
                scale_hack="Yes",
                rating=4,
                remarks="Tested through Level 6, scale hack works fine. Gyroscope drifts slightly to the right.",
                reported_by="@example",
                reported_at=_now_minus(20),
            ),
            dict(
                version_number="1.0.2",
                display_name="Monkey Ball",
                bundle_identifier="com.ooi.supermonkeybal",
                minimum_ios_version="2.0",
                touchhle_version="v0.2.3",
                operating_system="Android 13",
                gpu="Mali-G52 MC2",
                scale_hack="Yes",
                rating=5,
                remarks="The game runs perfectly, including scale hack and tilt controls.",
                reported_by="@example",
                reported_at=_now_minus(60),
            ),
            dict(
                version_number="1.0",
                display_name="SMB Lite",
                bundle_identifier="smblite",
                minimum_ios_version="2.0",
                touchhle_version="v0.2.1",
                operating_system="Windows 11",
                gpu="NVIDIA Quadro T2000",
                scale_hack="Yes",
                rating=5,
                remarks="Plays perfectly fine, tested until finishing Lite levels.",
                reported_by="@example",
                reported_at=_now_minus(180),
            ),
        ],
    },
    {
        "app": dict(
            name="Angry Birds",
            release_year=2009,
            developer_publisher="Rovio Entertainment / Chillingo",
            first_reported_by="@example",
            first_reported_at=_now_minus(50),
        ),
        "reports": [
            dict(
                version_number="1.0",
                display_name="Angry Birds",
                bundle_identifier="com.clickgamer.AngryBirds",
                minimum_ios_version="3.0",
                touchhle_version="dev (this fork)",
                operating_system="Android 14",
                gpu="Adreno 740",
                scale_hack="Yes",
                rating=4,
                remarks="Touch input works after the recent NSRunLoop fix. Can play through all Poached Eggs levels.",
                reported_by="@example",
                reported_at=_now_minus(2),
            ),
        ],
    },
    {
        "app": dict(
            name="Crystal Defenders: Vanguard Storm",
            release_year=2009,
            developer_publisher="MSF/Winds / Square Enix",
            first_reported_by="@example",
            first_reported_at=_now_minus(550),
        ),
        "reports": [
            dict(
                version_number="1.0",
                display_name="Vanguard",
                bundle_identifier="com.square-enix.VanguardSM",
                minimum_ios_version="2.2",
                touchhle_version="v0.2.2-723-g3207e84",
                operating_system="Android 15",
                gpu="Exynos",
                scale_hack="Yes",
                rating=5,
                remarks="Played a few stages and everything went smoothly.",
                reported_by="@example",
                reported_at=_now_minus(120),
            ),
            dict(
                version_number="1.0.1",
                display_name="Vanguard",
                bundle_identifier="com.square-enix.VanguardSM",
                minimum_ios_version="2.2",
                touchhle_version="v0.2.2-1515-97fc52f",
                operating_system="Android 15",
                gpu="Adreno",
                scale_hack="Couldn't test",
                rating=3,
                remarks="Wave 3 intro freezes the game. Options UI looks wrong.",
                reported_by="@example",
                reported_at=_now_minus(80),
            ),
        ],
    },
    {
        "app": dict(
            name="Doodle Jump",
            release_year=2009,
            developer_publisher="Lima Sky",
            first_reported_by="@example",
            first_reported_at=_now_minus(300),
        ),
        "reports": [
            dict(
                version_number="1.7",
                touchhle_version="v0.2.2",
                operating_system="Windows 10",
                gpu="Intel UHD 620",
                scale_hack="Yes",
                rating=2,
                remarks="Main menu loads but the game freezes after the first jump.",
                reported_by="@example",
                reported_at=_now_minus(40),
            ),
        ],
    },
    {
        "app": dict(
            name="Cube Runner",
            release_year=2008,
            developer_publisher="Andy Qua",
            first_reported_by="@example",
            first_reported_at=_now_minus(700),
        ),
        "reports": [
            dict(
                version_number="2.0",
                touchhle_version="v0.2.2",
                operating_system="macOS Sonoma 14.5",
                gpu="Apple M1",
                scale_hack="Yes",
                rating=5,
                remarks="Plays great, no issues.",
                reported_by="@example",
                reported_at=_now_minus(15),
            ),
        ],
    },
]


def seed() -> None:
    init_db()
    db = SessionLocal()
    try:
        if db.query(App).count() > 0:
            print("DB already has apps; skipping seed.")
            return
        for entry in SEED_APPS:
            app = App(**entry["app"])
            db.add(app)
            db.flush()
            for r in entry["reports"]:
                # Seed reports are pre-approved so a fresh deployment is not
                # empty (without a logged-in admin to approve them).
                db.add(Report(app_id=app.id, status=STATUS_APPROVED, **r))
        db.commit()
        print(f"Seeded {len(SEED_APPS)} apps with reports.")
    finally:
        db.close()


if __name__ == "__main__":
    seed()
