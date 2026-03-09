use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use sqlx::SqlitePool;
use tokio::io::AsyncWriteExt;

use crate::db;
use crate::errors::AppError;
use crate::models::{
    self, CreateVideoRequest, CreateVideoResponse, PlaybackInfo, ShareResponse,
    UploadSourceResponse, VideoStatusResponse, MAX_UPLOAD_BYTES,
};

// POST /api/videos
pub async fn create_video(
    State(pool): State<SqlitePool>,
    Json(body): Json<CreateVideoRequest>,
) -> Result<(StatusCode, Json<CreateVideoResponse>), AppError> {
    if body.title.is_empty() || body.filename.is_empty() || body.mime_type.is_empty() {
        return Err(AppError::BadRequest("Missing required fields".into()));
    }

    if body.size_bytes > MAX_UPLOAD_BYTES {
        return Err(AppError::PayloadTooLarge(format!(
            "File size {} exceeds maximum of {} bytes",
            body.size_bytes, MAX_UPLOAD_BYTES
        )));
    }

    if !models::is_supported_mime(&body.mime_type) {
        return Err(AppError::UnsupportedMedia(format!(
            "Unsupported MIME type: {}",
            body.mime_type
        )));
    }

    if !models::has_supported_extension(&body.filename) {
        return Err(AppError::UnsupportedMedia(format!(
            "Unsupported file extension: {}",
            body.filename
        )));
    }

    let video_id = models::generate_video_id();
    let share_token = models::generate_share_token();

    db::insert_video(
        &pool,
        &video_id,
        &body.title,
        &body.filename,
        body.size_bytes,
        &body.mime_type,
        &share_token,
    )
    .await?;

    tracing::info!(video_id = %video_id, "Video record created");

    Ok((
        StatusCode::CREATED,
        Json(CreateVideoResponse {
            video_id,
            share_url: format!("/s/{}", share_token),
            status: "uploading".into(),
            max_upload_bytes: MAX_UPLOAD_BYTES,
        }),
    ))
}

// PUT /api/videos/{video_id}/source
pub async fn upload_source(
    State(pool): State<SqlitePool>,
    Path(video_id): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<UploadSourceResponse>, AppError> {
    let video = db::get_video_by_id(&pool, &video_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Video {} not found", video_id)))?;

    match video.status.as_str() {
        "processing" | "playable" => {
            return Err(AppError::Conflict("SOURCE_ALREADY_UPLOADED".into()));
        }
        "failed" => {
            db::reset_video_for_reupload(&pool, &video_id).await?;
            tracing::info!(video_id = %video_id, "Reset failed video for re-upload");
        }
        "uploading" => {}
        _ => {
            return Err(AppError::Internal(format!(
                "Unexpected video status: {}",
                video.status
            )));
        }
    }

    let dir_path = format!("storage/originals/{}", video_id);
    tokio::fs::create_dir_all(&dir_path).await?;

    let ext = video
        .filename
        .rsplit('.')
        .next()
        .unwrap_or("mp4");
    let file_path = format!("{}/source.{}", dir_path, ext);

    let mut total_bytes: i64 = 0;
    let mut file = tokio::fs::File::create(&file_path).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name != "file" {
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
                    if total_bytes > MAX_UPLOAD_BYTES {
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

    db::update_video_upload_complete(&pool, &video_id, &file_path).await?;
    db::insert_job(&pool, &video_id, &file_path).await?;

    let now = models::now_iso();
    tracing::info!(
        video_id = %video_id,
        bytes = total_bytes,
        "Upload complete, job enqueued"
    );

    Ok(Json(UploadSourceResponse {
        video_id,
        status: "processing".into(),
        upload_completed_at: now,
    }))
}

// GET /api/videos/{video_id}/status
pub async fn get_video_status(
    State(pool): State<SqlitePool>,
    Path(video_id): Path<String>,
) -> Result<Json<VideoStatusResponse>, AppError> {
    let video = db::get_video_by_id(&pool, &video_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Video {} not found", video_id)))?;

    Ok(Json(VideoStatusResponse {
        video_id: video.id,
        status: video.status,
        processing_stage: video.processing_stage,
        share_url: format!("/s/{}", video.share_token),
        updated_at: video.updated_at,
        error_code: video.error_code,
        error_message: video.error_message,
    }))
}

// GET /api/share/{token}
pub async fn get_share(
    State(pool): State<SqlitePool>,
    Path(token): Path<String>,
) -> Result<Json<ShareResponse>, AppError> {
    let video = db::get_video_by_token(&pool, &token)
        .await?
        .ok_or_else(|| AppError::NotFound("SHARE_NOT_FOUND".into()))?;

    let hls_url = if video.status == "playable" {
        Some(format!("/media/hls/{}/master.m3u8", video.share_token))
    } else {
        None
    };

    Ok(Json(ShareResponse {
        video_id: video.id,
        title: video.title,
        status: video.status,
        processing_stage: video.processing_stage,
        playback: PlaybackInfo { hls_url },
        created_at: video.created_at,
    }))
}
