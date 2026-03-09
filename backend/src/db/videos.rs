use sqlx::SqlitePool;

use crate::models::VideoRow;
use crate::utils;

pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    filename: &str,
    size_bytes: i64,
    mime_type: &str,
    share_token: &str,
) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
    sqlx::query(
        "INSERT INTO videos (id, title, filename, size_bytes, mime_type, status, share_token, upload_initiated_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 'uploading', ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(title)
    .bind(filename)
    .bind(size_bytes)
    .bind(mime_type)
    .bind(share_token)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_by_id(
    pool: &SqlitePool,
    video_id: &str,
) -> Result<Option<VideoRow>, sqlx::Error> {
    sqlx::query_as::<_, VideoRow>("SELECT * FROM videos WHERE id = ?")
        .bind(video_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_by_token(
    pool: &SqlitePool,
    token: &str,
) -> Result<Option<VideoRow>, sqlx::Error> {
    sqlx::query_as::<_, VideoRow>("SELECT * FROM videos WHERE share_token = ?")
        .bind(token)
        .fetch_optional(pool)
        .await
}

pub async fn mark_upload_complete(
    pool: &SqlitePool,
    video_id: &str,
    source_path: &str,
) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
    sqlx::query(
        "UPDATE videos SET status = 'processing', processing_stage = 'queued', source_path = ?, upload_completed_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(source_path)
    .bind(&now)
    .bind(&now)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_status(
    pool: &SqlitePool,
    video_id: &str,
    status: &str,
    processing_stage: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
    sqlx::query(
        "UPDATE videos SET status = ?, processing_stage = ?, updated_at = ? WHERE id = ?",
    )
    .bind(status)
    .bind(processing_stage)
    .bind(&now)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_processing_started(
    pool: &SqlitePool,
    video_id: &str,
) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
    sqlx::query(
        "UPDATE videos SET processing_started_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&now)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_playable(pool: &SqlitePool, video_id: &str) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
    sqlx::query(
        "UPDATE videos SET status = 'playable', processing_stage = NULL, processing_completed_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(&now)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_failed(
    pool: &SqlitePool,
    video_id: &str,
    failure_stage: &str,
    failure_reason: &str,
) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
    sqlx::query(
        "UPDATE videos SET status = 'failed', processing_stage = NULL, failed_at = ?, failure_stage = ?, failure_reason = ?, error_code = 'PROCESSING_FAILED', error_message = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(failure_stage)
    .bind(failure_reason)
    .bind(failure_reason)
    .bind(&now)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn reset_for_reupload(
    pool: &SqlitePool,
    video_id: &str,
) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
    sqlx::query(
        "UPDATE videos SET status = 'uploading', processing_stage = NULL, source_path = NULL, error_code = NULL, error_message = NULL, upload_completed_at = NULL, processing_started_at = NULL, processing_completed_at = NULL, failed_at = NULL, failure_stage = NULL, failure_reason = NULL, updated_at = ? WHERE id = ?",
    )
    .bind(&now)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}
