use sqlx::SqlitePool;
use tokio::process::Command;

use crate::db;
use crate::models::JobRow;

const POLL_INTERVAL_MS: u64 = 1000;
const BACKOFF_SECONDS: &[i64] = &[10, 30, 90];

struct ProbeResult {
    height: u32,
    has_audio: bool,
}

pub async fn run(pool: SqlitePool) {
    let worker_id = format!("worker_{}", uuid::Uuid::new_v4().simple());
    tracing::info!(worker_id = %worker_id, "Worker started, polling for jobs");

    loop {
        match db::jobs::claim_pending(&pool, &worker_id).await {
            Ok(Some(job)) => {
                tracing::info!(job_id = %job.id, video_id = %job.video_id, "Claimed job");
                handle_job(&pool, job).await;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("Failed to poll for jobs: {}", e);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
}

async fn handle_job(pool: &SqlitePool, job: JobRow) {
    let stage = match process_job(pool, &job).await {
        Ok(()) => return,
        Err((stage, e)) => {
            tracing::error!(
                job_id = %job.id,
                video_id = %job.video_id,
                stage = %stage,
                "Job failed: {}",
                e
            );
            handle_failure(pool, &job, &stage, &e).await;
            stage
        }
    };
    let _ = stage;
}

async fn process_job(pool: &SqlitePool, job: &JobRow) -> Result<(), (String, String)> {
    let video_id = &job.video_id;
    let source_path = &job.source_path;

    db::videos::mark_processing_started(pool, video_id)
        .await
        .map_err(|e| ("probing".to_string(), format!("DB error: {}", e)))?;

    // Step 1: Probe
    db::videos::update_status(pool, video_id, "processing", Some("probing"))
        .await
        .map_err(|e| ("probing".to_string(), format!("DB error: {}", e)))?;

    let probe = run_ffprobe(source_path)
        .await
        .map_err(|e| ("probing".to_string(), e))?;

    tracing::info!(
        video_id = %video_id,
        height = probe.height,
        has_audio = probe.has_audio,
        "Probe complete"
    );

    // Step 2: Transcode
    db::videos::update_status(pool, video_id, "processing", Some("transcoding"))
        .await
        .map_err(|e| ("transcoding".to_string(), format!("DB error: {}", e)))?;

    let tmp_dir = format!("storage/hls/_tmp/{}", video_id);
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .map_err(|e| ("transcoding".to_string(), format!("Failed to create tmp dir: {}", e)))?;

    run_ffmpeg(source_path, &tmp_dir, &probe)
        .await
        .map_err(|e| ("transcoding".to_string(), e))?;

    tracing::info!(video_id = %video_id, "Transcode complete");

    // Step 3: Publish
    db::videos::update_status(pool, video_id, "processing", Some("publishing"))
        .await
        .map_err(|e| ("publishing".to_string(), format!("DB error: {}", e)))?;

    let video = db::videos::get_by_id(pool, video_id)
        .await
        .map_err(|e| ("publishing".to_string(), format!("DB error: {}", e)))?
        .ok_or_else(|| ("publishing".to_string(), "Video not found during publish".to_string()))?;

    let public_dir = format!("storage/hls/public/{}", video.share_token);
    if tokio::fs::metadata(&public_dir).await.is_ok() {
        tokio::fs::remove_dir_all(&public_dir)
            .await
            .map_err(|e| ("publishing".to_string(), format!("Failed to clean old public dir: {}", e)))?;
    }

    tokio::fs::rename(&tmp_dir, &public_dir)
        .await
        .map_err(|e| ("publishing".to_string(), format!("Failed to publish: {}", e)))?;

    tracing::info!(video_id = %video_id, share_token = %video.share_token, "Published to public path");

    // Step 4: Mark done
    db::videos::mark_playable(pool, video_id)
        .await
        .map_err(|e| ("publishing".to_string(), format!("DB error: {}", e)))?;
    db::jobs::mark_done(pool, &job.id)
        .await
        .map_err(|e| ("publishing".to_string(), format!("DB error: {}", e)))?;

    // Step 5: Cleanup source
    let source_dir = format!("storage/originals/{}", video_id);
    if let Err(e) = tokio::fs::remove_dir_all(&source_dir).await {
        tracing::warn!(video_id = %video_id, "Failed to delete source dir: {}", e);
    }

    tracing::info!(video_id = %video_id, "Job complete — video is playable");

    Ok(())
}

async fn handle_failure(pool: &SqlitePool, job: &JobRow, stage: &str, error: &str) {
    if job.attempt_count < job.max_attempts {
        let backoff_idx = (job.attempt_count as usize).saturating_sub(1).min(BACKOFF_SECONDS.len() - 1);
        let backoff = BACKOFF_SECONDS[backoff_idx];
        tracing::warn!(
            job_id = %job.id,
            attempt = job.attempt_count,
            max = job.max_attempts,
            backoff_s = backoff,
            "Rescheduling job"
        );
        let _ = db::jobs::reschedule(pool, &job.id, error, backoff).await;
    } else {
        tracing::error!(
            job_id = %job.id,
            video_id = %job.video_id,
            "Final failure — marking video as failed"
        );
        let _ = db::videos::mark_failed(pool, &job.video_id, stage, error).await;
        let _ = db::jobs::mark_failed(pool, &job.id, error).await;

        let source_dir = format!("storage/originals/{}", job.video_id);
        if let Err(e) = tokio::fs::remove_dir_all(&source_dir).await {
            tracing::warn!(video_id = %job.video_id, "Failed to delete source on final failure: {}", e);
        }
    }
}

// --- ffprobe ---

async fn run_ffprobe(source_path: &str) -> Result<ProbeResult, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
            source_path,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to execute ffprobe: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse ffprobe JSON: {}", e))?;

    let streams = json["streams"]
        .as_array()
        .ok_or("No streams found in ffprobe output")?;

    let video_stream = streams
        .iter()
        .find(|s| s["codec_type"].as_str() == Some("video"))
        .ok_or("No video stream found")?;

    let height = video_stream["height"]
        .as_u64()
        .ok_or("No height in video stream")? as u32;

    let has_audio = streams
        .iter()
        .any(|s| s["codec_type"].as_str() == Some("audio"));

    Ok(ProbeResult { height, has_audio })
}

// --- ffmpeg ---

async fn run_ffmpeg(
    source_path: &str,
    tmp_dir: &str,
    probe: &ProbeResult,
) -> Result<(), String> {
    if probe.height >= 720 {
        transcode_dual_rendition(source_path, tmp_dir, probe.has_audio).await
    } else if probe.height >= 360 {
        transcode_rendition(source_path, tmp_dir, 360, probe.has_audio).await?;
        generate_master_playlist(tmp_dir, &["360p"]).await
    } else {
        transcode_rendition(source_path, tmp_dir, probe.height, probe.has_audio).await?;
        generate_master_playlist(tmp_dir, &[&format!("{}p", probe.height)]).await
    }
}

/// Single-pass dual-rendition: decodes source once, splits into 720p + 360p.
/// Uses `split`/`asplit` in filter_complex so each variant gets its own stream,
/// avoiding the "Same elementary stream" and "Unable to map stream at a:1" HLS errors.
async fn transcode_dual_rendition(
    source_path: &str,
    tmp_dir: &str,
    has_audio: bool,
) -> Result<(), String> {
    tokio::fs::create_dir_all(format!("{}/0", tmp_dir))
        .await
        .map_err(|e| format!("Failed to create variant dir 0: {}", e))?;
    tokio::fs::create_dir_all(format!("{}/1", tmp_dir))
        .await
        .map_err(|e| format!("Failed to create variant dir 1: {}", e))?;

    let mut filter = String::from(
        "[0:v]split=2[v720][v360];\
         [v720]scale=-2:720[out720];\
         [v360]scale=-2:360[out360]"
    );
    if has_audio {
        filter.push_str(";[0:a]asplit=2[a0][a1]");
    }

    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        source_path.to_string(),
        "-filter_complex".to_string(),
        filter,
        "-map".to_string(), "[out720]".to_string(),
        "-map".to_string(), "[out360]".to_string(),
    ];

    if has_audio {
        args.extend([
            "-map".to_string(), "[a0]".to_string(),
            "-map".to_string(), "[a1]".to_string(),
        ]);
    }

    args.extend([
        "-c:v".to_string(), "libx264".to_string(),
        "-preset".to_string(), "ultrafast".to_string(),
        "-crf".to_string(), "28".to_string(),
    ]);

    if has_audio {
        args.extend([
            "-c:a".to_string(), "aac".to_string(),
            "-b:a".to_string(), "128k".to_string(),
            "-ac".to_string(), "2".to_string(),
        ]);
    }

    let var_stream_map = if has_audio {
        "v:0,a:0 v:1,a:1"
    } else {
        "v:0 v:1"
    };

    args.extend([
        "-f".to_string(), "hls".to_string(),
        "-hls_time".to_string(), "4".to_string(),
        "-hls_playlist_type".to_string(), "vod".to_string(),
        "-hls_segment_filename".to_string(),
        format!("{}/%v/seg_%03d.ts", tmp_dir),
        "-var_stream_map".to_string(), var_stream_map.to_string(),
        "-master_pl_name".to_string(), "master.m3u8".to_string(),
        format!("{}/%v/index.m3u8", tmp_dir),
    ]);

    tracing::info!("Transcoding dual rendition (720p + 360p) single-pass");
    run_ffmpeg_command(&args).await?;

    // ffmpeg outputs to numeric dirs (0/, 1/); rename to named renditions
    tokio::fs::rename(format!("{}/0", tmp_dir), format!("{}/720p", tmp_dir))
        .await
        .map_err(|e| format!("Failed to rename variant 0 -> 720p: {}", e))?;
    tokio::fs::rename(format!("{}/1", tmp_dir), format!("{}/360p", tmp_dir))
        .await
        .map_err(|e| format!("Failed to rename variant 1 -> 360p: {}", e))?;

    // Fix master playlist: replace numeric refs with named renditions
    let master_path = format!("{}/master.m3u8", tmp_dir);
    let content = tokio::fs::read_to_string(&master_path)
        .await
        .map_err(|e| format!("Failed to read master playlist: {}", e))?;
    let content = content
        .replace("0/index.m3u8", "720p/index.m3u8")
        .replace("1/index.m3u8", "360p/index.m3u8");
    tokio::fs::write(&master_path, content)
        .await
        .map_err(|e| format!("Failed to rewrite master playlist: {}", e))?;

    tracing::info!("Dual rendition transcode complete");
    Ok(())
}

async fn transcode_rendition(
    source_path: &str,
    tmp_dir: &str,
    target_height: u32,
    has_audio: bool,
) -> Result<(), String> {
    let rendition_name = format!("{}p", target_height);
    let rendition_dir = format!("{}/{}", tmp_dir, rendition_name);
    tokio::fs::create_dir_all(&rendition_dir)
        .await
        .map_err(|e| e.to_string())?;

    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        source_path.to_string(),
        "-vf".to_string(),
        format!("scale=-2:{}", target_height),
        "-c:v".to_string(),
        "libx264".to_string(),
        "-preset".to_string(),
        "ultrafast".to_string(),
        "-crf".to_string(),
        "28".to_string(),
    ];

    if has_audio {
        args.extend([
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "128k".to_string(),
            "-ac".to_string(),
            "2".to_string(),
        ]);
    } else {
        args.push("-an".to_string());
    }

    args.extend([
        "-f".to_string(),
        "hls".to_string(),
        "-hls_time".to_string(),
        "4".to_string(),
        "-hls_playlist_type".to_string(),
        "vod".to_string(),
        "-hls_segment_filename".to_string(),
        format!("{}/seg_%03d.ts", rendition_dir),
        format!("{}/index.m3u8", rendition_dir),
    ]);

    tracing::info!(rendition = %rendition_name, "Transcoding rendition");
    run_ffmpeg_command(&args).await?;
    tracing::info!(rendition = %rendition_name, "Rendition complete");

    Ok(())
}

async fn run_ffmpeg_command(args: &[String]) -> Result<(), String> {
    tracing::debug!("ffmpeg {}", args.join(" "));

    let output = Command::new("ffmpeg")
        .args(args)
        .output()
        .await
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last_lines: String = stderr
            .lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("ffmpeg exit {}: {}", output.status, last_lines));
    }

    Ok(())
}

// --- Playlist helpers ---

async fn generate_master_playlist(tmp_dir: &str, renditions: &[&str]) -> Result<(), String> {
    let mut content = String::from("#EXTM3U\n");

    for rendition in renditions {
        // Bandwidth is approximate — the player will adapt based on actual segment sizes
        let bandwidth = match *rendition {
            "720p" => 2500000,
            "360p" => 500000,
            _ => 800000,
        };
        content.push_str(&format!(
            "#EXT-X-STREAM-INF:BANDWIDTH={}\n{}/index.m3u8\n",
            bandwidth, rendition
        ));
    }

    let path = format!("{}/master.m3u8", tmp_dir);
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| format!("Failed to write master playlist: {}", e))
}

