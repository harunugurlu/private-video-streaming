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

### ffmpeg — 720p rendition

```bash
ffmpeg -y \
  -i "storage/originals/{video_id}/source.mp4" \
  -vf "scale=-2:720" \
  -c:v libx264 -preset fast -crf 28 \
  -c:a aac -b:a 128k -ac 2 \
  -f hls \
  -hls_time 4 \
  -hls_playlist_type vod \
  -hls_segment_filename "storage/hls/_tmp/{video_id}/720p/seg_%03d.ts" \
  "storage/hls/_tmp/{video_id}/720p/index.m3u8"
```

### ffmpeg — 360p rendition

```bash
ffmpeg -y \
  -i "storage/originals/{video_id}/source.mp4" \
  -vf "scale=-2:360" \
  -c:v libx264 -preset fast -crf 28 \
  -c:a aac -b:a 128k -ac 2 \
  -f hls \
  -hls_time 4 \
  -hls_playlist_type vod \
  -hls_segment_filename "storage/hls/_tmp/{video_id}/360p/seg_%03d.ts" \
  "storage/hls/_tmp/{video_id}/360p/index.m3u8"
```

### Master playlist (generated by worker, not ffmpeg)

Written to `storage/hls/_tmp/{video_id}/master.m3u8`:

```
#EXTM3U
#EXT-X-STREAM-INF:BANDWIDTH=2500000
720p/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=500000
360p/index.m3u8
```

After all renditions and the master playlist are written, the worker atomically renames the tmp directory to the public path:

```
storage/hls/_tmp/{video_id}/  ->  storage/hls/public/{share_token}/
```
