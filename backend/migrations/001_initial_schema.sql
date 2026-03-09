PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS videos (
    id                      TEXT PRIMARY KEY,
    title                   TEXT NOT NULL,
    filename                TEXT NOT NULL,
    size_bytes              INTEGER NOT NULL,
    mime_type               TEXT NOT NULL,
    status                  TEXT NOT NULL DEFAULT 'uploading',
    share_token             TEXT UNIQUE NOT NULL,
    processing_stage        TEXT,
    source_path             TEXT,
    error_code              TEXT,
    error_message           TEXT,
    upload_initiated_at     TEXT NOT NULL,
    upload_completed_at     TEXT,
    processing_started_at   TEXT,
    processing_completed_at TEXT,
    failed_at               TEXT,
    failure_stage           TEXT,
    failure_reason          TEXT,
    created_at              TEXT NOT NULL,
    updated_at              TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_videos_share_token ON videos(share_token);

CREATE TABLE IF NOT EXISTS jobs (
    id              TEXT PRIMARY KEY,
    type            TEXT NOT NULL,
    video_id        TEXT NOT NULL REFERENCES videos(id),
    source_path     TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',
    attempt_count   INTEGER NOT NULL DEFAULT 0,
    max_attempts    INTEGER NOT NULL DEFAULT 3,
    scheduled_at    TEXT NOT NULL,
    locked_at       TEXT,
    worker_id       TEXT,
    last_error      TEXT,
    idempotency_key TEXT UNIQUE NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_status_scheduled ON jobs(status, scheduled_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_idempotency ON jobs(idempotency_key);
