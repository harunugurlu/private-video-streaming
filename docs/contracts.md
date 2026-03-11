# System Contracts (API, Worker, State, Storage)

## Purpose

This document defines the contracts for implementation: API endpoints, worker/job behavior, state transitions, and storage conventions.

## 1) UI Route and Polling Contract

### Routes

- `/upload`: uploader flow (create video + upload source file).
- `/watch/{token}`: shared page used by both uploader and viewers for progress + playback.

### Polling Strategy

- `/upload` may poll `GET /api/videos/{video_id}/status` during/after upload.
- `/watch/{token}` polls `GET /api/share/{token}` every 2-3 seconds.
- Polling stops when status becomes `playable` or `failed`.

### Upload Progress

Upload progress is tracked client-side only via browser XHR/fetch progress events. The status API does not expose upload byte progress. The share page shows a generic "uploading" state for viewers who open the link during upload.

### Shared Page Behavior (`/watch/{token}`)

- `uploading`: show upload-in-progress UI and optional progress.
- `processing`: show current `processing_stage`.
- `playable`: initialize player with `playback.hls_url`.
- `failed`: show failure message.

## 2) API Contracts

Base path: `/api`

### 2.1 `POST /api/videos`

Purpose:
- Create video record before byte upload.
- Issue share token immediately.
- Perform fast pre-checks using client-provided metadata.

Request body:

```json
{
  "title": "My video",
  "filename": "demo.mp4",
  "size_bytes": 123456789,
  "mime_type": "video/mp4"
}
```

Response body (201):

```json
{
  "video_id": "vid_123",
  "share_token": "shr_abc123",
  "status": "uploading",
  "max_upload_bytes": 1073741824
}
```

Supported formats:
- Accepted MIME types: `video/mp4`, `video/quicktime` (MOV), `video/webm`, `video/x-msvideo` (AVI), `video/x-matroska` (MKV).
- Accepted extensions: `.mp4`, `.mov`, `.webm`, `.avi`, `.mkv`.
- These are guardrails; the worker's `ffprobe` step is the true validation gate.

Validation errors:
- `413` when `size_bytes > 1073741824`.
- `415` for MIME type or extension outside the supported list.
- `400` for malformed payload.

### 2.2 `PUT /api/videos/{video_id}/source`

Purpose:
- Receive source bytes as multipart upload.
- Persist source file to local storage.
- Mark upload complete and enqueue processing job.

Request content type:
- `multipart/form-data` with single file field: `file`.

Upload-size enforcement:
- Server enforces actual uploaded size `<= 1GB` during `PUT /source`.
- If streamed bytes exceed limit, upload is aborted and `413` is returned.

Success response (200):

```json
{
  "video_id": "vid_123",
  "status": "processing",
  "upload_completed_at": "2026-03-08T12:34:56Z"
}
```

Conflict/retry rules:
- If video status is `processing` or `playable`: return `409 SOURCE_ALREADY_UPLOADED`.
- If video status is `failed`: allow re-upload for the same `video_id` (manual retry path).
- If prior upload is incomplete: allow retry for same `video_id`.
- For accepted retries, clear prior failure metadata and reset state to `uploading` before handling new bytes.

### 2.3 `GET /api/videos/{video_id}/status`

Purpose:
- Uploader/internal status endpoint.

Response body (200):

```json
{
  "video_id": "vid_123",
  "status": "uploading",
  "processing_stage": null,
  "share_token": "shr_abc123",
  "updated_at": "2026-03-08T12:34:56Z",
  "error_code": null,
  "error_message": null
}
```

Notes:
- `processing_stage`: one of `queued | probing | transcoding | publishing | null`. Set to `queued` when the job is first enqueued (in the `PUT /source` handler), before the worker picks it up. The frontend maps stages to deterministic progress-bar positions (e.g., queued ~5%, probing ~15%, transcoding ~50%, publishing ~90%).

### 2.4 `GET /api/share/{token}`

Purpose:
- Viewer/share-link endpoint.
- Resolve share token and return current state + playback metadata.

Response body (200):

```json
{
  "video_id": "vid_123",
  "title": "My video",
  "status": "processing",
  "processing_stage": "transcoding",
  "playback": {
    "hls_url": null
  },
  "created_at": "2026-03-08T12:00:00Z"
}
```

Rules:
- If status is not `playable`, return status payload with `playback.hls_url = null`.
- Invalid or unknown token: `404 SHARE_NOT_FOUND`.

### 2.5 `GET /media/hls/{share_token}/{path...}`

Purpose:
- Serve published HLS files from public media directory.

Rules:
- Missing/unpublished file returns `404` (no internal details).
- No cache headers for MVP.
- Safety checks required to block path traversal.

Note: Media URLs use the same `share_token` that identifies the video in the share page. Since anyone with the share link already receives the `hls_url` via the API, a separate media identifier adds no real security boundary. This keeps the system simpler — one unguessable token per video. Example URL: `/media/hls/shr_abc123/master.m3u8`.

## 3) Why Use Two Endpoints for Video Upload (`POST /videos` + `PUT /videos/{id}/source`)

- Keeps the backend lifecycle explicit.
- Enables early validation and immediate share token issuance before byte transfer.
- Improves failure handling with a durable `video_id` and resumable context. Video record is inserted before upload.
- Produces cleaner status transitions/metrics (`uploading -> processing -> playable/failed`).
- Allows future migration to direct-to-object-storage upload without redesigning lifecycle APIs.

Important trust note:
- Metadata from `POST /api/videos` is for early checks/UX.
- Server re-validates actual uploaded content during `PUT /source` and worker probe.

## 4) Worker and Job Contract (DB-backed Queue)

### Job Model

Minimal job fields:
- `job_id`
- `type` (`process_video`)
- `video_id`
- `source_path`
- `status` (`pending | in_progress | done | failed`)
- `attempt_count`
- `max_attempts`
- `scheduled_at`
- `locked_at`
- `worker_id`
- `last_error`
- `idempotency_key`

### Worker Runtime Policy (MVP Locked)

- Run mode: same Rust project, separate worker process (`--worker`).
- Why this was chosen: still simple to run, but cleaner than an in-process thread because API and worker crashes are isolated.
- Worker count: `1` worker process.
- Worker concurrency: `1` job at a time.
- Retry policy: max `3` attempts with backoff `10s -> 30s -> 90s`.
- Lock timeout: `15 minutes`.
- Heartbeat: worker refreshes `locked_at` every 30-60 seconds while processing.
- Job poll interval: `1 second`. Worst-case job pickup latency is ~1s, negligible relative to total processing time. SQLite handles 1 query/second trivially.
- Final failure handling: delete source file immediately.
- Processing progress mode: stage-based only (`processing_stage`), no estimated percent in MVP.

### Worker Behavior

1. Claim one pending job atomically.
2. Update video status to `processing` if needed. Set `processing_started_at` timestamp.
3. Set `processing_stage=probing`; run `ffprobe` to validate source and extract metadata (duration, resolution, codecs).
4. Set `processing_stage=transcoding`; run `ffmpeg` to produce multi-rendition HLS output in tmp path. Rendition ladder: 720p + 360p (no upscaling; skip any rendition above source resolution; if source < 360p, single rendition at source resolution).
5. Set `processing_stage=publishing`; atomically publish tmp output to public path.
6. Mark video `playable`, set `processing_completed_at`, and mark job done.
7. Delete source file from `storage/originals/{video_id}/` (no longer needed after successful publish).

Failure/retry behavior:
- Retry with backoff until `max_attempts`.
- On final failure: set video `failed`, set `failed_at`, `failure_stage`, `failure_reason`, and delete source file (prevent disk from filling).
- Source files are deleted on both success (step 7) and final failure.
- Enforce idempotency for `process_video` per `video_id`.

### Worker Topology Notes (Architecture)

- MVP topology uses SQLite + polling for queue consumption (SQLite has no pub/sub queue notifications).
- This is intentionally simple for local development and zipped submission.
- Designed to scale with minimal refactor: keep same contracts, migrate queue DB to PostgreSQL, and increase worker process count.
- Supporting more parallel jobs later will be a configuration change (`WORKER_CONCURRENCY`) plus operational tuning (CPU limits/ffmpeg threads).

## 5) Video State Machine Contract

Allowed states:
- `uploading`
- `processing`
- `playable`
- `failed`

Allowed transitions:
- `uploading -> processing` (source upload persisted + job enqueued)
- `processing -> playable` (successful publish)
- `processing -> failed` (final processing failure)
- `failed -> uploading` (manual retry path for re-upload)

Notes:
- Worker retries happen while video is still in `processing`.
- `processing -> failed` is only for exhausted retries/final failure.

Invalid transitions must be rejected.

## 6) File Layout and Publish Contract

Directory layout:
- Source upload: `storage/originals/{video_id}/source.ext`
- HLS tmp output: `storage/hls/_tmp/{video_id}/...`
- HLS public output: `storage/hls/public/{share_token}/...`

Expected HLS artifacts (multi-rendition):
- `master.m3u8` (variant playlist referencing all renditions)
- `720p/index.m3u8`, `720p/seg_000.ts`, ...
- `360p/index.m3u8`, `360p/seg_000.ts`, ...

Rendition rules:
- No upscaling: skip any rendition above source resolution.
- If source >= 720p: produce 720p + 360p.
- If source >= 360p but < 720p: produce 360p only.
- If source < 360p: single rendition at source resolution.

Publish rule:
- Use atomic directory rename/move from tmp to public on the same filesystem.
- Do not expose tmp directory through HTTP.

## 7) Database Schema

SQLite stores timestamps as TEXT in ISO 8601 format. When migrating to PostgreSQL, these become `TIMESTAMPTZ` columns.

### `videos` table

```sql
CREATE TABLE videos (
    id              TEXT PRIMARY KEY,                -- prefixed, e.g. "vid_..."
    title           TEXT NOT NULL,
    filename        TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    mime_type       TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'uploading', -- uploading | processing | playable | failed
    share_token     TEXT UNIQUE NOT NULL,             -- prefixed, e.g. "shr_..."
    processing_stage TEXT,                            -- queued | probing | transcoding | publishing | NULL
    source_path     TEXT,
    error_code      TEXT,
    error_message   TEXT,
    upload_initiated_at    TEXT NOT NULL,
    upload_completed_at    TEXT,
    processing_started_at  TEXT,
    processing_completed_at TEXT,
    failed_at       TEXT,
    failure_stage   TEXT,
    failure_reason  TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_videos_share_token ON videos(share_token);
```

### `jobs` table

```sql
CREATE TABLE jobs (
    id              TEXT PRIMARY KEY,
    type            TEXT NOT NULL,                    -- e.g. "process_video"
    video_id        TEXT NOT NULL REFERENCES videos(id),
    source_path     TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending',  -- pending | in_progress | done | failed
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

CREATE INDEX idx_jobs_status_scheduled ON jobs(status, scheduled_at);
CREATE UNIQUE INDEX idx_jobs_idempotency ON jobs(idempotency_key);
```
