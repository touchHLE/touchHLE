# HyperHLE app compatibility database

A community-run compatibility database for [HyperHLE](https://github.com/j92580498-max/touchHLE)
(a community fork of the touchHLE iPhone OS emulator), modelled on the
original [appdb.touchhle.org](https://appdb.touchhle.org/).

Anyone can submit a compatibility report for an app they've tested in
HyperHLE: app name, version, OS, GPU, rating (1–5⭐), remarks, screenshot
(uploaded as a file). The site aggregates reports per app and per app
version, and can pre-fill the form from an uploaded HyperHLE log.

## Stack

- **FastAPI** + **Jinja2** (server-rendered HTML, no SPA)
- **SQLite** via SQLAlchemy 2.x
- Plain CSS, no build step

## Run locally

```bash
cd appdb
python -m venv .venv
source .venv/bin/activate
pip install -e .
uvicorn app.main:app --host 0.0.0.0 --port 8000 --reload
```

Then open <http://localhost:8000/>. The database is auto-created and seeded
with a handful of example apps the first time the server starts.

The SQLite file lives at `appdb/appdb.sqlite3` in development; in
production (Fly.io with a volume) it lives at `/data/appdb.sqlite3`.
Uploaded screenshots live at `appdb/uploads/` in development and at
`/data/uploads/` in production.

## Layout

```
appdb/
├── pyproject.toml         # dependencies + package metadata
├── README.md
└── app/
    ├── main.py            # FastAPI routes
    ├── db.py              # SQLAlchemy models, engine, init_db
    ├── seed.py            # demo data
    ├── log_parser.py      # HyperHLE / touchHLE log → form fields
    ├── templates/         # Jinja2 templates
    │   ├── base.html
    │   ├── index.html         # Apps list + per-rating stats
    │   ├── app_detail.html    # Per-app: versions + reports table
    │   ├── submit_report.html # Form to add a compatibility report
    │   └── about.html
    └── static/
        └── style.css
```

## Routes

| Method | Path | Description |
| --- | --- | --- |
| GET | `/` | Apps list + stats, optional `?q=` search |
| GET | `/apps/{id}` | App detail page (versions + reports) |
| GET | `/submit` | Compatibility report form |
| POST | `/submit` | Create a new report (and optionally a new app) |
| POST | `/submit/parse-log` | Parse uploaded HyperHLE log → re-render the form pre-filled |
| GET | `/about` | Rating scale and house rules |
| GET | `/uploads/{filename}` | Static — serves uploaded screenshots |
| GET | `/healthz` | Liveness check |

## Deploying

The app is deployable as a FastAPI backend (e.g. to Fly.io with a 1 GB
volume mounted at `/data` for both the SQLite file and uploaded
screenshots).

## Notes

- HyperHLE is a community fork of touchHLE; this database is **not**
  affiliated with upstream touchHLE.
- Reports are anonymous — there is no login. To prevent spam in
  production, add a CAPTCHA or rate-limiting before exposing it widely.
