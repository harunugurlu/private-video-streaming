# StreamDrop — Private Video Streaming Service

A minimal private video streaming service built with Rust and Svelte. Users upload videos anonymously, receive a shareable link immediately, and the system transcodes the video in the background into adaptive HLS for browser playback.

## How It Works

1. User opens the upload page, selects a video, and submits
2. The system returns a shareable link before the upload even completes
3. A background worker transcodes the video into multi-rendition HLS (720p + 360p)
4. Anyone with the link can watch processing progress and stream the video once ready

The primary design goal is **minimizing time-to-stream** — the time from starting an upload until the video is playable.

## Prerequisites

- **Rust** (1.85+ for edition 2024)
- **Node.js** (v18+)
- **ffmpeg** and **ffprobe** (must be available on PATH)

## Quick Start

### 1. Backend

Start the API server and worker in two separate terminals:

```bash
cd backend
cargo run                # API server on http://localhost:3000
cargo run -- --worker    # Background processing worker
```

The API server creates the SQLite database and storage directories automatically on first run.

### 2. Frontend

```bash
cd frontend
npm install
npm run dev              # Dev server on http://localhost:5173
```

The Vite dev server proxies `/api` and `/media` requests to the backend at port 3000.

### 3. Use

1. Open http://localhost:5173/upload
2. Enter a title, drag-and-drop or browse for a video file
3. Click "Upload & Share" — you'll be redirected to the watch page
4. Share the URL with anyone — they can monitor progress and watch once ready

## Project Structure

```
backend/
  src/
    main.rs          # Entrypoint — API server and worker launcher
    handlers.rs      # Axum route handlers (thin layer)
    service.rs       # Business logic (validation, upload, status)
    worker.rs        # Background job processor (ffprobe, ffmpeg, HLS)
    config.rs        # Constants (upload limits, supported formats)
    errors.rs        # Typed error responses
    dto.rs           # Request/response types
    utils.rs         # ID generation, timestamps
    db/              # Database layer (videos, jobs)
    models/          # DB row types (VideoRow, JobRow)
  migrations/        # SQL schema

frontend/
  src/
    routes/
      upload/        # Upload page
      watch/[token]/ # Share/watch page with HLS player
    lib/
      api.ts         # API client with upload progress
      types.ts       # Shared types and constants
      components/    # VideoPlayer, UploadDropzone, ProcessingProgress

docs/
  architecture.md    # Core architecture decisions and system design
  contracts.md       # API contracts, state machine, DB schema
```

## Architecture

See [docs/architecture.md](docs/architecture.md) for the full design document covering:

- System container diagram
- Key architecture decisions with rationale
- Time-to-stream optimization strategy
- Cost efficiency
- Horizontal scaling path

See [docs/contracts.md](docs/contracts.md) for detailed API contracts, video state machine, worker behavior, and database schema.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Backend | Rust (Axum, SQLx, Tokio) |
| Frontend | SvelteKit 5, TypeScript, hls.js |
| Database | SQLite (WAL mode) |
| Transcoding | ffmpeg / ffprobe |
| Playback | HLS with adaptive bitrate |

## API Endpoints

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/api/videos` | Create video record, get share token |
| `PUT` | `/api/videos/{id}/source` | Upload video file (multipart) |
| `GET` | `/api/videos/{id}/status` | Check processing status |
| `GET` | `/api/share/{token}` | Resolve share link for viewers |
| `GET` | `/media/hls/{token}/...` | Serve HLS playlists and segments |

## Key Design Decisions

- **Two-step upload** — Metadata first (`POST`), then bytes (`PUT`). Enables immediate share token issuance and future migration to presigned URL uploads.
- **DB-backed job queue** — API enqueues work and returns immediately. Worker processes asynchronously with retry and backoff.
- **HLS with ABR** — 720p + 360p renditions for consistent playback regardless of network conditions or file size.
- **SQLite for MVP** — No separate database server. Schema designed for straightforward PostgreSQL migration when scaling.
- **Single-pass dual-rendition transcoding** — Decodes source once, outputs both quality levels simultaneously via ffmpeg filter graphs.
