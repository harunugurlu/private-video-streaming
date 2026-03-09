# Step 3 - System Contracts (API, Worker, State, Storage)

## Purpose

This document defines the contracts for implementation: API endpoints, worker/job behavior, state transitions, and storage conventions.

## 1) UI Route and Polling Contract

### Routes

- `/upload`: uploader flow (create video + upload source file).
- `/s/{token}`: shared page used by both uploader and viewers for progress + playback.

### Polling Strategy

- `/upload` may poll `GET /api/videos/{video_id}/status` during/after upload.
- `/s/{token}` polls `GET /api/share/{token}` every 2-3 seconds.
- Polling stops when status becomes `playable` or `failed`.

### Shared Page Behavior (`/s/{token}`)

- `uploading`: show upload-in-progress UI and optional progress.
- `processing`: show current `processing_stage`.
- `playable`: initialize player with `playback.hls_url`.
- `failed`: show failure message.

## 2) API Contracts

Base path: `/api`

### 2.1 `POST /api/videos`

Purpose:
- Create video record before byte upload.
- Issue share URL immediately.
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
  "share_url": "/s/shr_abc123",
  "status": "uploading",
  "max_upload_bytes": 1073741824
}
```

Validation errors:
- `413` when `size_bytes > 1073741824`.
- `415` for unsupported media type.
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
  "progress_percent": null,
  "share_url": "/s/shr_abc123",
  "updated_at": "2026-03-08T12:34:56Z",
  "error_code": null,
  "error_message": null
}
```

Notes:
- `processing_stage`: one of `queued | probing | transcoding | packaging | publishing | null`.
- `progress_percent`: nullable; for MVP processing is stage-based, so this is typically `null` during processing.

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
  "progress_percent": null,
  "playback": {
    "hls_url": null
  },
  "created_at": "2026-03-08T12:00:00Z"
}
```

Rules:
- If status is not `playable`, return status payload with `playback.hls_url = null`.
- Invalid or unknown token: `404 SHARE_NOT_FOUND`.

### 2.5 `GET /media/hls/{playback_key}/{path...}`

Purpose:
- Serve published HLS files from public media directory.

Rules:
- Missing/unpublished file returns `404` (no internal details).
- No cache headers for MVP.
- Safety checks required to block path traversal.

### 2.6 `playback_key` Rationale

- `playback_key` is a separate, unguessable public identifier used in media URLs (instead of exposing internal `video_id` values).
- It reduces trivial URL enumeration and decouples public playback paths from internal database IDs.
- It is stored in the `videos` table as a unique field and used to build paths like `/media/hls/{playback_key}/master.m3u8`.
- This is hardening/obfuscation, not full authorization.

## 3) Why Use Two Endpoints for Video Upload (`POST /videos` + `PUT /videos/{id}/source`)

- Keeps the backend lifecycle explicit.
- Enables early validation and immediate share-link issuance before byte transfer.
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
- Final failure handling: delete source file immediately.
- Processing progress mode: stage-based only (`processing_stage`), no estimated percent in MVP.

### Worker Behavior

1. Claim one pending job atomically.
2. Update video status to `processing` if needed.
3. Set `processing_stage=probing`; run `ffprobe`.
4. Set `processing_stage=transcoding`; run `ffmpeg` to produce HLS output in tmp path.
5. Set `processing_stage=publishing`; atomically publish tmp output to public path.
6. Mark video `playable`, set `processing_completed_at`, and mark job done.

Failure/retry behavior:
- Retry with backoff until `max_attempts`.
- On final failure: set video `failed`, set `failed_at`, `failure_stage`, `failure_reason`, and delete source file.
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
- HLS public output: `storage/hls/public/{playback_key}/...`

Expected HLS artifacts:
- `master.m3u8`
- per rendition playlist, e.g. `720p/index.m3u8`
- segments, e.g. `720p/seg_000.ts`

Publish rule:
- Use atomic directory rename/move from tmp to public on the same filesystem.
- Do not expose tmp directory through HTTP.
- No upscaling: first playable rendition target is `min(source_height, 720)`.
