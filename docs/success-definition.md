# Private Video Streaming Project Success Definition

## Overview

This document defines what "success" means for the 1-week take-home implementation of a minimal private video streaming service.

## 1) Product Outcome

The user starts an upload, immediately receives a shareable link, can open that link to monitor progress, and can start playback as soon as processing is complete.

## 2) Primary Success Metrics

### Link and Progress UX

- Shareable link is issued in `<= 1s` after upload initialization.
- Progress page is accessible immediately after upload initialization.

### Time-to-Stream Metrics

To avoid mixing network speed with backend performance, success is measured with two metrics:

- End-to-end time-to-stream (upload started -> first playable stream), measured with a reference profile:
  - Reference file/network: `500MB MP4`, `20 Mbps` uplink.
  - Target: `p50 <= 4 min`, `p95 <= 6 min`.
- System processing latency (upload completed -> first playable stream):
  - Target: `p50 <= 20s`, `p95 <= 60s`.

### Playback Quality Floor

- Playback starts within `2-3s` on an average home network.
- Adaptive bitrate streaming is used to minimize buffering.
- `720p` is acceptable as the initial quality target.

### Reliability

- Successful processing rate for valid files under 1GB: `>= 98%`.

## 3) Scope and Constraints

- Max upload size: `1GB`.
- Anonymous uploads are supported.
- Each upload has a shareable link.
- Cost optimization is considered, but not a strict success gate in this phase.

## 4) Day-1 Format Strategy

- Guaranteed support (MVP): `MP4 (H.264/AAC)`.
- Next formats (best effort): `MOV`, `WebM`.
- Uploads with unsupported codec/container are failed with explicit processing error reasons; this is surfaced on status/share pages.

## 5) MVP Capacity Target

- `5` concurrent uploads.
- `50` concurrent viewers.

## 6) Lifecycle Assumptions (Architecture Decisions)

- Shareable link is created at upload initialization, after lightweight checks (size/type guardrails).
- Video becomes playable after upload completion and processing; no "stream while upload is still in progress" in MVP.
- MVP upload flow is browser -> backend API -> local file storage to keep local development and project submission simple.
- Storage integration stays behind a clear abstraction so migration to object storage/direct upload remains straightforward after MVP.

## 7) Out of Scope (This Phase)

- Authentication and authorization.
- Infrastructure-as-code.
- Advanced cost tuning and multi-region delivery.
