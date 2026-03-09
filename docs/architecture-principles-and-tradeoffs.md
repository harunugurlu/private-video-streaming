# Architecture Principles and Tradeoffs

## Purpose

This document captures the architectural principles and high-impact implementation decisions that guide the MVP build.

## Architecture Principles

- Prioritize shipping MVP quickly while keeping time-to-stream optimization as a guiding objective.
- Prefer fewer moving parts in MVP if they still satisfy core success metrics.
- Keep request/response APIs fast; push long-running media work to asynchronous workers.
- Design retry-safe workflows with strict idempotency on critical transitions.
- Make progress visible to users (status-driven UX) instead of blocking flows.
- Instrument only fundamental pipeline metrics first, then expand after MVP validation.

## Locked Implementation Choices

- Upload path (MVP): browser multipart upload to backend API, persisted to local file storage.
- Processing model: asynchronous background processing with status updates.
- Playback protocol: HLS for MVP.
- Rendition strategy: multi-rendition HLS (720p + 360p, no upscaling) for adaptive bitrate playback.
- Transcoding toolchain: ffmpeg for transcoding and HLS packaging, ffprobe for source validation and metadata extraction.
- Database: SQLite for MVP — no separate process, simple deployment, sufficient write throughput for MVP capacity (5 concurrent uploads, 1 worker).
- Reliability guardrail: strict idempotency for source-upload finalization (`PUT /api/videos/{id}/source`) and worker job consumption.

## Tradeoff Matrix

- Chosen: local file storage via backend upload for MVP.
  - Gain: Faster and simpler.
  - Tradeoff: backend handles upload bandwidth; horizontal scaling is weaker than direct-to-object-storage.
- Chosen: asynchronous processing pipeline.
  - Gain: fast API responses, resilient retries, clear progress UX.
  - Tradeoff: queue/state complexity versus synchronous flow.
- Chosen: HLS-only playback in MVP.
  - Gain: reduced protocol complexity and faster delivery.
  - Tradeoff: broader multi-protocol compatibility deferred.
- Chosen: multi-rendition HLS (720p + 360p) for adaptive bitrate playback.
  - Gain: consistent playback regardless of network conditions and file size.
  - Tradeoff: slightly longer transcoding time compared to single-rendition output.
- Chosen: SQLite for MVP database.
  - Gain: no separate database server, simple deployment, zero operational cost.
  - Tradeoff: limited concurrent write performance; migrate to PostgreSQL for horizontal scaling.
- Chosen: idempotency-first for critical events.
  - Gain: safe retries and fewer duplicate-processing failures.
  - Tradeoff: additional state guards and uniqueness constraints in backend design.

## Cost Efficiency

- SQLite eliminates the cost of running a separate database server.
- Local file storage eliminates cloud storage costs during MVP operation.
- Rust provides low memory and CPU overhead per request, reducing infrastructure requirements.
- Single binary deployment (API and worker from the same Rust crate) minimizes operational complexity.
- ffmpeg is open-source with no licensing cost.

## Planned Evolution After MVP

- Replace local file storage with object storage.
- Move upload data path to direct browser upload using presigned URLs.
- Keep API contracts and status model stable while swapping storage implementation.

## Minimal Observability Specification

### Core Metrics

- Link issue latency (`POST /api/videos` request start -> share link returned).
- End-to-end time-to-stream (upload start -> stream playable).
- Processing latency (upload completed -> processing completed).
- Failure rate by stage (`probing`, `transcoding`, `publishing`).
- State distribution and state duration (`uploading`, `processing`, `playable`, `failed`).

### Minimal Data Needed Per Video

- `upload_initiated_at`
- `upload_completed_at`
- `processing_started_at`
- `processing_completed_at`
- `failed_at`
- `status`
- `failure_stage` (nullable)
- `failure_reason` (nullable)
