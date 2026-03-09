use axum::extract::Multipart;
use sqlx::SqlitePool;
use tokio::io::AsyncWriteExt;

use crate::config;
use crate::db;
use crate::dto::{
    CreateVideoRequest, CreateVideoResponse, PlaybackInfo, ShareResponse, UploadSourceResponse,
    VideoStatusResponse,
};
use crate::errors::AppError;
use crate::models::VideoRow;
use crate::utils;

pub async fn create_video(
    pool: &SqlitePool,
    request: CreateVideoRequest,
) -> Result<CreateVideoResponse, AppError> {
    validate_create_request(&request)?;

    let video_id = utils::generate_video_id();
    let share_token = utils::generate_share_token();

    db::videos::insert(
        pool,
        &video_id,
        &request.title,
        &request.filename,
        request.size_bytes,
        &request.mime_type,
        &share_token,
    )
    .await?;

    tracing::info!(video_id = %video_id, "Video record created");

    Ok(CreateVideoResponse {
        video_id,
        share_token,
        status: "uploading".into(),
        max_upload_bytes: config::MAX_UPLOAD_BYTES,
    })
}

pub async fn upload_source(
    pool: &SqlitePool,
    video_id: &str,
    multipart: Multipart,
) -> Result<UploadSourceResponse, AppError> {
    let video = get_video_or_404(pool, video_id).await?;
    validate_upload_state(&video)?;

    if video.status == "failed" {
        db::videos::reset_for_reupload(pool, video_id).await?;
        tracing::info!(video_id = %video_id, "Reset failed video for re-upload");
    }

    let file_path = persist_upload_file(&video, multipart).await?;

    db::videos::mark_upload_complete(pool, video_id, &file_path).await?;
    db::jobs::insert(pool, video_id, &file_path).await?;

    tracing::info!(video_id = %video_id, "Upload complete, job enqueued");

    Ok(UploadSourceResponse {
        video_id: video_id.to_string(),
        status: "processing".into(),
        upload_completed_at: utils::now_iso(),
    })
}

pub async fn get_video_status(
    pool: &SqlitePool,
    video_id: &str,
) -> Result<VideoStatusResponse, AppError> {
    let video = get_video_or_404(pool, video_id).await?;

    Ok(VideoStatusResponse {
        video_id: video.id,
        status: video.status,
        processing_stage: video.processing_stage,
        share_token: video.share_token,
        updated_at: video.updated_at,
        error_code: video.error_code,
        error_message: video.error_message,
    })
}

pub async fn get_share(pool: &SqlitePool, token: &str) -> Result<ShareResponse, AppError> {
    let video = db::videos::get_by_token(pool, token)
        .await?
        .ok_or_else(|| AppError::NotFound("SHARE_NOT_FOUND".into()))?;

    let hls_url = if video.status == "playable" {
        Some(format!("/media/hls/{}/master.m3u8", video.share_token))
    } else {
        None
    };

    Ok(ShareResponse {
        video_id: video.id,
        title: video.title,
        status: video.status,
        processing_stage: video.processing_stage,
        playback: PlaybackInfo { hls_url },
        created_at: video.created_at,
    })
}

// --- Private helpers ---

fn validate_create_request(request: &CreateVideoRequest) -> Result<(), AppError> {
    if request.title.is_empty() || request.filename.is_empty() || request.mime_type.is_empty() {
        return Err(AppError::BadRequest("Missing required fields".into()));
    }

    if request.size_bytes > config::MAX_UPLOAD_BYTES {
        return Err(AppError::PayloadTooLarge(format!(
            "File size {} exceeds maximum of {} bytes",
            request.size_bytes, config::MAX_UPLOAD_BYTES
        )));
    }

    if !config::is_supported_mime(&request.mime_type) {
        return Err(AppError::UnsupportedMedia(format!(
            "Unsupported MIME type: {}",
            request.mime_type
        )));
    }

    if !config::has_supported_extension(&request.filename) {
        return Err(AppError::UnsupportedMedia(format!(
            "Unsupported file extension: {}",
            request.filename
        )));
    }

    Ok(())
}

fn validate_upload_state(video: &VideoRow) -> Result<(), AppError> {
    match video.status.as_str() {
        "processing" | "playable" => {
            Err(AppError::Conflict("SOURCE_ALREADY_UPLOADED".into()))
        }
        "uploading" | "failed" => Ok(()),
        other => Err(AppError::Internal(format!(
            "Unexpected video status: {}",
            other
        ))),
    }
}

async fn get_video_or_404(pool: &SqlitePool, video_id: &str) -> Result<VideoRow, AppError> {
    db::videos::get_by_id(pool, video_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Video {} not found", video_id)))
}

async fn persist_upload_file(
    video: &VideoRow,
    mut multipart: Multipart,
) -> Result<String, AppError> {
    let dir_path = format!("storage/originals/{}", video.id);
    tokio::fs::create_dir_all(&dir_path).await?;

    let ext = video.filename.rsplit('.').next().unwrap_or("mp4");
    let file_path = format!("{}/source.{}", dir_path, ext);

    let mut total_bytes: i64 = 0;
    let mut file = tokio::fs::File::create(&file_path).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        if field.name().unwrap_or("") != "file" {
            continue;
        }

        let mut stream = field;
        loop {
            let chunk = stream
                .chunk()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to read chunk: {}", e)))?;

            match chunk {
                Some(bytes) => {
                    total_bytes += bytes.len() as i64;
                    if total_bytes > config::MAX_UPLOAD_BYTES {
                        drop(file);
                        let _ = tokio::fs::remove_file(&file_path).await;
                        return Err(AppError::PayloadTooLarge(
                            "Uploaded file exceeds 1GB limit".into(),
                        ));
                    }
                    file.write_all(&bytes).await?;
                }
                None => break,
            }
        }
        break;
    }

    if total_bytes == 0 {
        let _ = tokio::fs::remove_file(&file_path).await;
        return Err(AppError::BadRequest("No file data received".into()));
    }

    file.flush().await?;

    tracing::info!(video_id = %video.id, bytes = total_bytes, "File persisted to disk");

    Ok(file_path)
}
