# ffmpeg Processing Profile

## Purpose

This document defines the ffprobe and ffmpeg commands used by the processing worker, with a parameter-by-parameter explanation of every flag. It serves as both an implementation reference and a tuning guide.

## ffprobe — Source Validation and Metadata Extraction

```bash
ffprobe -v quiet -print_format json -show_format -show_streams input.mp4
```

| Parameter | What it does | Why |
|---|---|---|
| `-v quiet` | Suppresses log output, only prints the requested data. | Keeps output clean for JSON parsing. |
| `-print_format json` | Output in JSON format. | Easy to parse in Rust (serde_json). |
| `-show_format` | Includes container-level info: duration, size, format name, bitrate. | Needed to determine total duration (for rendition decisions) and validate the container. |
| `-show_streams` | Includes per-stream info: codec, resolution, frame rate, audio channels. | Needed to read source resolution (height) for rendition selection, and to detect unsupported codecs. |

### Key fields to extract

- `streams[video].height` — source resolution, used for rendition ladder selection.
- `streams[video].codec_name` — source video codec (e.g., `h264`, `vp9`).
- `format.duration` — total duration in seconds.
- `streams[audio]` — presence check; if no audio stream exists, skip audio mapping.

## ffmpeg — Multi-Rendition HLS Transcoding

### Dual-rendition command (source >= 720p)

```bash
ffmpeg -i input.mp4 \
  -filter_complex "[0:v]split=2[v720][v360];[v720]scale=-2:720[out720];[v360]scale=-2:360[out360]" \
  -map "[out720]" -map "[out360]" -map 0:a? \
  -c:v libx264 -preset fast -crf 28 \
  -c:a aac -b:a 128k -ac 2 \
  -f hls \
  -hls_time 4 \
  -hls_playlist_type vod \
  -hls_segment_filename "%v/seg_%03d.ts" \
  -var_stream_map "v:0,a:0 v:1,a:1" \
  -master_pl_name master.m3u8 \
  "%v/index.m3u8"
```

### Single-rendition command (source < 720p)

When the source is below 720p, only one rendition is produced (at 360p if source >= 360p, or at source resolution if source < 360p).

```bash
ffmpeg -i input.mp4 \
  -vf "scale=-2:360" \
  -c:v libx264 -preset fast -crf 28 \
  -c:a aac -b:a 128k -ac 2 \
  -f hls \
  -hls_time 4 \
  -hls_playlist_type vod \
  -hls_segment_filename "360p/seg_%03d.ts" \
  -hls_flags single_file \
  "360p/index.m3u8"
```

For single-rendition output, the worker generates a `master.m3u8` manually (a trivial variant playlist pointing to the single stream playlist).

## Parameter Reference

### Input and stream mapping

| Parameter | What it does | Why this value |
|---|---|---|
| `-i input.mp4` | Specifies the input file. | The source video to transcode. |
| `-filter_complex "[0:v]split=2..."` | Takes the first video stream (`[0:v]`), duplicates it into two copies, and scales each to a target height. | Produces 720p and 360p from a single decode pass, avoiding decoding the source twice. |
| `scale=-2:720` | Scales video to 720px height. Width is auto-calculated to preserve aspect ratio, rounded to the nearest even number. | `-2` (not `-1`) ensures the width is even, which is required by libx264. |
| `-map "[out720]" -map "[out360]"` | Selects the two scaled video streams for output. | Without explicit `-map`, ffmpeg would auto-select only one stream. |
| `-map 0:a?` | Maps the first audio stream from input. The `?` makes it optional — if no audio stream exists, ffmpeg continues without error. | Some video files have no audio track; `?` prevents a hard failure in that case. |

### Video encoding

| Parameter | What it does | Why this value |
|---|---|---|
| `-c:v libx264` | Video codec: H.264 via the x264 encoder. | Universal browser support. Every HLS player handles H.264. This is the safe default. |
| `-preset fast` | Controls the encoding speed vs compression efficiency tradeoff. Options range from `ultrafast` (fastest, worst compression) to `veryslow` (slowest, best compression). | `fast` is a good balance: roughly 2x slower than `ultrafast` but produces ~20% smaller files at the same quality. Since time-to-stream matters, we avoid `medium` or slower. |
| `-crf 28` | Constant Rate Factor — controls output quality. Scale is 0 (lossless) to 51 (worst). Lower values produce better quality but larger files. | Default is 23. Using 28 produces files ~40% smaller than CRF 23, at the cost of some visual quality loss. This is deliberate: the assignment prioritizes time-to-stream over high video quality. Smaller output files also mean faster segment delivery to viewers. |

### Audio encoding

| Parameter | What it does | Why this value |
|---|---|---|
| `-c:a aac` | Audio codec: AAC. | Universal browser support, pairs with H.264 in HLS. |
| `-b:a 128k` | Audio bitrate: 128 kbps. | Standard for web delivery. Transparent quality for speech and music, small overhead relative to video. |
| `-ac 2` | Downmix to stereo (2 channels). | Some source files have 5.1 or other multi-channel audio. Most browsers and devices play stereo. Avoids compatibility issues. |

### HLS output

| Parameter | What it does | Why this value |
|---|---|---|
| `-f hls` | Output format: HLS. | Tells ffmpeg to produce `.m3u8` playlists and `.ts` transport stream segment files. |
| `-hls_time 4` | Target segment duration in seconds. Each `.ts` file covers approximately this many seconds of video. | 4 seconds is the industry standard. Shorter (2s) = faster initial playback start but more HTTP requests and slightly worse compression. Longer (10s) = fewer requests but slower start and coarser seek granularity. 4s balances all factors. |
| `-hls_playlist_type vod` | Marks the playlist as Video on Demand. | Adds `#EXT-X-ENDLIST` to the playlist so the player knows the full segment list upfront. Without this, the player would treat it as a live stream and keep polling for new segments. |
| `-hls_segment_filename "%v/seg_%03d.ts"` | Naming pattern for segment files. `%v` is replaced by the variant index (mapped to directory names via `var_stream_map`). | Produces files like `720p/seg_000.ts`, `720p/seg_001.ts`, etc. Keeps segments organized per rendition. |
| `-var_stream_map "v:0,a:0 v:1,a:1"` | Defines variant streams for ABR. Maps video stream 0 + audio stream 0 as one variant, video stream 1 + audio stream 1 as another. | This is what makes the master playlist list multiple quality levels. Each space-separated group becomes a variant in the master playlist. |
| `-master_pl_name master.m3u8` | Name of the master (variant) playlist file. | The top-level file the HLS player loads first. It lists all available renditions with their bandwidth and resolution metadata so the player can choose which one to start with. |
| `"%v/index.m3u8"` | Output path pattern for per-rendition playlists. `%v` is the variant index. | Produces `720p/index.m3u8` and `360p/index.m3u8`. |

## Rendition Selection Logic

The worker decides which renditions to produce based on the source video height from `ffprobe`:

| Source height | Renditions produced | Rationale |
|---|---|---|
| >= 720px | 720p + 360p | Full ABR ladder. Player can switch between high and low quality. |
| >= 360px, < 720px | 360p only | No upscaling. Single rendition at the highest quality below source. |
| < 360px | Source resolution only | No upscaling. Preserve original quality. |

No upscaling is ever performed — producing a rendition at a higher resolution than the source wastes CPU time and disk space with no quality benefit.

## Tuning Notes

If processing is too slow (time-to-stream targets not met):

1. **First lever: `-preset ultrafast`** — roughly 2x faster than `fast`, but output files are ~20% larger (worse compression at same CRF). Larger files mean slightly slower segment delivery, but the encoding speed gain usually dominates.
2. **Second lever: increase CRF** — e.g., CRF 32 instead of 28. Produces lower visual quality but encodes faster and produces smaller files.
3. **Third lever: drop to single rendition** — skip the 360p rendition for very large files to cut encoding time. ABR is lost but time-to-stream improves.

If quality is too low:

1. **Lower CRF** — e.g., CRF 23 (ffmpeg default). Produces better quality but larger files and slower encoding.
2. **Use `-preset medium`** — slower encoding but better compression efficiency (same quality at smaller file sizes).
