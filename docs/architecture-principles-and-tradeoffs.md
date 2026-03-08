# Step 2 - Architecture Principles and Tradeoffs

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
- Reliability guardrail: strict idempotency for `complete-upload` and worker job consumption.

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
- Chosen: idempotency-first for critical events.
  - Gain: safe retries and fewer duplicate-processing failures.
  - Tradeoff: additional state guards and uniqueness constraints in backend design.

## Planned Evolution After MVP

- Replace local file storage with object storage.
- Move upload data path to direct browser upload using presigned URLs.
- Keep API contracts and status model stable while swapping storage implementation.

## Minimal Observability Specification

### Core Metrics

- Link issue latency (`init-upload` request start -> share link returned).
- End-to-end time-to-stream (upload start -> stream playable).
- Processing latency (upload completed -> processing completed).
- Failure rate by stage (`validation`, `transcode`, `packaging`).
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
