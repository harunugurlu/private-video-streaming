# Testing Guide

## Prerequisites

- Rust installed (`cargo --version`)
- ffmpeg installed (`ffmpeg -version`, `ffprobe -version`)
- A sample video file (any .mp4 will work)

## Starting the Services

Open two terminals, both in `backend/`:

```bash
# Terminal 1 — API server
cargo run

# Terminal 2 — Worker process
cargo run -- --worker
```

The API server listens on `http://localhost:3000`. The worker polls for jobs every 1 second.

## API Testing

### 1. Health check

```bash
curl http://localhost:3000/health
```

Expected: `ok`

### 2. Create a video record

```bash
curl -X POST http://localhost:3000/api/videos \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Sample Video",
    "filename": "sample-5s.mp4",
    "size_bytes": 2848208,
    "mime_type": "video/mp4"
  }'
```

Expected (201):

```json
{
  "video_id": "vid_...",
  "share_token": "shr_...",
  "status": "uploading",
  "max_upload_bytes": 1073741824
}
```

Save the `video_id` and `share_token` for subsequent requests.

### 3. Check status (before upload)

```bash
curl http://localhost:3000/api/videos/{video_id}/status
```

Expected: `status: "uploading"`, `processing_stage: null`.

### 4. Upload the source file

```bash
curl -X PUT http://localhost:3000/api/videos/{video_id}/source \
  -F "file=@C:/Users/harunugurlu/Desktop/sample-5s.mp4"
```

Expected (200):

```json
{
  "video_id": "vid_...",
  "status": "processing",
  "upload_completed_at": "2026-..."
}
```

### 5. Check status (during/after processing)

```bash
curl http://localhost:3000/api/videos/{video_id}/status
```

Expected progression:
- Immediately after upload: `status: "processing"`, `processing_stage: "queued"`
- During processing: `processing_stage` changes through `"probing"` -> `"transcoding"` -> `"publishing"`
- After completion: `status: "playable"`, `processing_stage: null`

### 6. Check share endpoint

```bash
curl http://localhost:3000/api/share/{share_token}
```

Expected (when playable):

```json
{
  "video_id": "vid_...",
  "title": "Sample Video",
  "status": "playable",
  "processing_stage": null,
  "playback": {
    "hls_url": "/media/hls/{share_token}/master.m3u8"
  },
  "created_at": "2026-..."
}
```

### 7. Verify HLS output

```bash
curl http://localhost:3000/media/hls/{share_token}/master.m3u8
```

Expected:

```
#EXTM3U
#EXT-X-STREAM-INF:BANDWIDTH=2500000
720p/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=500000
360p/index.m3u8
```

You can also check individual rendition playlists:

```bash
curl http://localhost:3000/media/hls/{share_token}/720p/index.m3u8
curl http://localhost:3000/media/hls/{share_token}/360p/index.m3u8
```

## Error Case Testing

### File too large (413)

```bash
curl -X POST http://localhost:3000/api/videos \
  -H "Content-Type: application/json" \
  -d '{"title":"Big","filename":"big.mp4","size_bytes":2000000000,"mime_type":"video/mp4"}'
```

Expected: 413, `"File size 2000000000 exceeds maximum of 1073741824 bytes"`

### Unsupported format (415)

```bash
curl -X POST http://localhost:3000/api/videos \
  -H "Content-Type: application/json" \
  -d '{"title":"Bad","filename":"bad.txt","size_bytes":1000,"mime_type":"text/plain"}'
```

Expected: 415, `"Unsupported MIME type: text/plain"`

### Unknown share token (404)

```bash
curl http://localhost:3000/api/share/shr_nonexistent
```

Expected: 404, `"SHARE_NOT_FOUND"`

### Duplicate upload (409)

Upload a file to a video that is already in `processing` or `playable` state:

```bash
curl -X PUT http://localhost:3000/api/videos/{video_id}/source \
  -F "file=@C:/Users/harunugurlu/Desktop/sample-5s.mp4"
```

Expected: 409, `"SOURCE_ALREADY_UPLOADED"`

## Clean Reset

To start fresh (delete all data and uploaded files):

```bash
# From backend/ directory — make sure both processes are stopped first
rm data.db data.db-wal data.db-shm
rm -rf storage/
```

PowerShell equivalent:

```powershell
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue data.db, data.db-wal, data.db-shm, storage
```

## Exact ffmpeg Commands Used

These are the exact commands the worker executes. You can run them manually to debug.

### ffprobe (source validation)

```bash
ffprobe -v quiet -print_format json -show_format -show_streams "storage/originals/{video_id}/source.mp4"
```

Key output fields the worker reads:
- `streams[?].codec_type == "video"` -> `.height` (for rendition selection)
- `streams[?].codec_type == "audio"` (presence check)

### ffmpeg — single-pass dual rendition (720p + 360p, source >= 720p)

For sources >= 720p, the worker uses a single ffmpeg call with `split`/`asplit` to avoid decoding the source twice:

```bash
ffmpeg -y \
  -i "storage/originals/{video_id}/source.mp4" \
  -filter_complex "[0:v]split=2[v720][v360];[v720]scale=-2:720[out720];[v360]scale=-2:360[out360];[0:a]asplit=2[a0][a1]" \
  -map "[out720]" -map "[out360]" -map "[a0]" -map "[a1]" \
  -c:v libx264 -preset fast -crf 28 \
  -c:a aac -b:a 128k -ac 2 \
  -f hls \
  -hls_time 4 \
  -hls_playlist_type vod \
  -hls_segment_filename "storage/hls/_tmp/{video_id}/%v/seg_%03d.ts" \
  -var_stream_map "v:0,a:0 v:1,a:1" \
  -master_pl_name master.m3u8 \
  "storage/hls/_tmp/{video_id}/%v/index.m3u8"
```

After ffmpeg completes, the worker renames `0/` -> `720p/` and `1/` -> `360p/`, and rewrites `master.m3u8` references to match.

For sources without audio, omit the `asplit` filter, the audio `-map` entries, the audio codec flags, and use `-var_stream_map "v:0 v:1"`.

### ffmpeg — single rendition (source < 720p)

For sources between 360p and 720p, or below 360p, a single ffmpeg call produces one rendition:

```bash
ffmpeg -y \
  -i "storage/originals/{video_id}/source.mp4" \
  -vf "scale=-2:{height}" \
  -c:v libx264 -preset fast -crf 28 \
  -c:a aac -b:a 128k -ac 2 \
  -f hls \
  -hls_time 4 \
  -hls_playlist_type vod \
  -hls_segment_filename "storage/hls/_tmp/{video_id}/{height}p/seg_%03d.ts" \
  "storage/hls/_tmp/{video_id}/{height}p/index.m3u8"
```

### Master playlist

For dual rendition, ffmpeg generates `master.m3u8` via `-master_pl_name` (then the worker fixes the directory names). For single rendition, the worker writes `master.m3u8` directly.

After processing, the worker atomically renames the tmp directory to the public path:

```
storage/hls/_tmp/{video_id}/  ->  storage/hls/public/{share_token}/
```

## Test Scenarios

### Functional — Happy Path

| # | Scenario | Steps | Expected |
|---|----------|-------|----------|
| F1 | Small file upload + playback | Upload a small MP4 (<50 MB) via browser, wait for processing, play video | Full flow completes: upload -> processing stages -> playable -> HLS plays in browser |
| F2 | Large file upload | Upload a 200-500 MB MP4 via browser | Progress bar shows accurate percentage, time-to-stream meets targets |
| F3 | Share link in second browser | After F1, open the share link in a different browser/incognito | Viewer sees processing progress or playable video without uploading |
| F4 | Copy link button | Click "Copy link" on the watch page | URL is copied to clipboard |

### ABR (Adaptive Bitrate)

| # | Scenario | Steps | Expected |
|---|----------|-------|----------|
| A1 | Dual rendition output | Upload a 1080p or 720p video, inspect `master.m3u8` | Lists both `720p/index.m3u8` and `360p/index.m3u8` |
| A2 | Quality switching under throttle | Play a video, open DevTools -> Network, throttle to "Slow 3G" | Player switches from 720p `.ts` segments to 360p segments |
| A3 | Quality upgrade on fast network | While throttled (A2), switch back to "Online" or "Fast 3G" | Player switches back to 720p segments |
| A4 | Single rendition for low-res source | Upload a video with resolution 360-719p | `master.m3u8` lists only `360p/index.m3u8` |
| A5 | No upscaling for tiny source | Upload a video < 360p resolution | Single rendition at source resolution, no upscaling |

### Malformed / Edge-Case Uploads

| # | Scenario | Steps | Expected |
|---|----------|-------|----------|
| M1 | Corrupted file | Rename a .txt file to .mp4, upload it | ffprobe fails, job retries 3x, video marked `failed`, UI shows "Processing failed" |
| M2 | No audio track | Upload a video with no audio stream | Worker uses `-an` flag, output plays without audio |
| M3 | Very short video | Upload a 1-2 second video | Valid HLS output with 1 segment, plays correctly |
| M4 | Non-standard codec | Upload a VP9/WebM file | ffmpeg transcodes to H.264 HLS, plays in browser |
| M5 | Large file near 1 GB | Upload a file just under 1 GB | Upload completes, processing works end-to-end |

### Validation / Error Handling

| # | Scenario | Steps | Expected |
|---|----------|-------|----------|
| V1 | File too large (API) | POST `/api/videos` with `size_bytes` > 1 GB | 413 response |
| V2 | File too large (upload) | PUT a file > 1 GB to `/api/videos/{id}/source` | 413 response, server enforces limit |
| V3 | Unsupported MIME type | POST `/api/videos` with `mime_type: "text/plain"` | 415 response |
| V4 | Unsupported extension | POST `/api/videos` with `filename: "video.exe"` | 415 response |
| V5 | Unknown share token | GET `/api/share/shr_nonexistent` | 404 response |
| V6 | Duplicate upload | PUT source to a video already in `processing` or `playable` state | 409 response |
| V7 | Bad JSON body | POST `/api/videos` with malformed JSON | 400 response |
| V8 | Missing required fields | POST `/api/videos` with empty title | 400 response |
| V9 | Idempotent job creation | Upload source twice rapidly (race condition) | Only one job created (idempotency key prevents duplicates) |

### Security

| # | Scenario | Steps | Expected |
|---|----------|-------|----------|
| S1 | Path traversal on HLS | GET `/media/hls/../../etc/passwd` | 404 or 400, no file leak (`ServeDir` blocks traversal) |
| S2 | Path traversal on share token | GET `/api/share/../../etc/passwd` | 404 `SHARE_NOT_FOUND` |
| S3 | Oversized multipart body | Send a multipart body exceeding the 1 GB + 1 MB limit | Connection rejected by axum body limit layer |
| S4 | Filename with path traversal | POST `/api/videos` with `filename: "../../../etc/shadow"` | File saved safely under `storage/originals/{video_id}/`, filename ignored for path |

### Performance

| # | Scenario | Steps | Expected |
|---|----------|-------|----------|
| P1 | Time-to-stream for 500 MB file | Upload 500 MB MP4, measure time from upload start to first playable frame | p50 <= 4 min (per success-definition.md) |
| P2 | Processing latency | Measure time from upload complete to `playable` status | p50 <= 20s, p95 <= 60s for typical files |
| P3 | Playback start time | Load the watch page for a playable video, measure time to first frame | 2-3 seconds |
| P4 | Single-pass vs dual-pass | Compare processing time for a 30-min 1080p source using single-pass (current) vs per-rendition (old) | Single-pass should be 10-30% faster |
