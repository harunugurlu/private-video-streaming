use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::models::{self, JobRow, VideoRow};

pub async fn init_pool(database_url: &str) -> SqlitePool {
    let options = SqliteConnectOptions::from_str(database_url)
        .expect("Invalid database URL")
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .expect("Failed to connect to SQLite");

    run_migrations(&pool).await;

    pool
}

async fn run_migrations(pool: &SqlitePool) {
    let sql = include_str!("../migrations/001_initial_schema.sql");
    for statement in sql.split(';') {
        let trimmed = statement.trim();
        if trimmed.is_empty() {
            continue;
        }
        sqlx::query(trimmed)
            .execute(pool)
            .await
            .expect("Failed to run migration");
    }
    tracing::info!("Database migrations applied");
}

// --- Video queries ---

pub async fn insert_video(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    filename: &str,
    size_bytes: i64,
    mime_type: &str,
    share_token: &str,
) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
    sqlx::query(
        "INSERT INTO videos (id, title, filename, size_bytes, mime_type, status, share_token, upload_initiated_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 'uploading', ?, ?, ?, ?)"
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

pub async fn get_video_by_id(
    pool: &SqlitePool,
    video_id: &str,
) -> Result<Option<VideoRow>, sqlx::Error> {
    sqlx::query_as::<_, VideoRow>("SELECT * FROM videos WHERE id = ?")
        .bind(video_id)
        .fetch_optional(pool)
        .await
}

pub async fn get_video_by_token(
    pool: &SqlitePool,
    token: &str,
) -> Result<Option<VideoRow>, sqlx::Error> {
    sqlx::query_as::<_, VideoRow>("SELECT * FROM videos WHERE share_token = ?")
        .bind(token)
        .fetch_optional(pool)
        .await
}

pub async fn update_video_upload_complete(
    pool: &SqlitePool,
    video_id: &str,
    source_path: &str,
) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
    sqlx::query(
        "UPDATE videos SET status = 'processing', processing_stage = 'queued', source_path = ?, upload_completed_at = ?, updated_at = ? WHERE id = ?"
    )
    .bind(source_path)
    .bind(&now)
    .bind(&now)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_video_status(
    pool: &SqlitePool,
    video_id: &str,
    status: &str,
    processing_stage: Option<&str>,
) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
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

pub async fn update_video_processing_started(
    pool: &SqlitePool,
    video_id: &str,
) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
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

pub async fn mark_video_playable(
    pool: &SqlitePool,
    video_id: &str,
) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
    sqlx::query(
        "UPDATE videos SET status = 'playable', processing_stage = NULL, processing_completed_at = ?, updated_at = ? WHERE id = ?"
    )
    .bind(&now)
    .bind(&now)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_video_failed(
    pool: &SqlitePool,
    video_id: &str,
    failure_stage: &str,
    failure_reason: &str,
) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
    sqlx::query(
        "UPDATE videos SET status = 'failed', processing_stage = NULL, failed_at = ?, failure_stage = ?, failure_reason = ?, error_code = 'PROCESSING_FAILED', error_message = ?, updated_at = ? WHERE id = ?"
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

pub async fn reset_video_for_reupload(
    pool: &SqlitePool,
    video_id: &str,
) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
    sqlx::query(
        "UPDATE videos SET status = 'uploading', processing_stage = NULL, source_path = NULL, error_code = NULL, error_message = NULL, upload_completed_at = NULL, processing_started_at = NULL, processing_completed_at = NULL, failed_at = NULL, failure_stage = NULL, failure_reason = NULL, updated_at = ? WHERE id = ?"
    )
    .bind(&now)
    .bind(video_id)
    .execute(pool)
    .await?;
    Ok(())
}

// --- Job queries ---

pub async fn insert_job(
    pool: &SqlitePool,
    video_id: &str,
    source_path: &str,
) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
    let job_id = models::generate_job_id();
    let idempotency_key = format!("process_video:{}", video_id);

    sqlx::query(
        "INSERT INTO jobs (id, type, video_id, source_path, status, scheduled_at, idempotency_key, created_at, updated_at)
         VALUES (?, 'process_video', ?, ?, 'pending', ?, ?, ?, ?)
         ON CONFLICT(idempotency_key) DO NOTHING"
    )
    .bind(&job_id)
    .bind(video_id)
    .bind(source_path)
    .bind(&now)
    .bind(&idempotency_key)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn claim_pending_job(
    pool: &SqlitePool,
    worker_id: &str,
) -> Result<Option<JobRow>, sqlx::Error> {
    let now = models::now_iso();

    let result = sqlx::query_as::<_, JobRow>(
        "UPDATE jobs SET status = 'in_progress', locked_at = ?, worker_id = ?, attempt_count = attempt_count + 1, updated_at = ?
         WHERE id = (SELECT id FROM jobs WHERE status = 'pending' ORDER BY scheduled_at ASC LIMIT 1)
         RETURNING *"
    )
    .bind(&now)
    .bind(worker_id)
    .bind(&now)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

pub async fn mark_job_done(pool: &SqlitePool, job_id: &str) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
    sqlx::query("UPDATE jobs SET status = 'done', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(job_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_job_failed(
    pool: &SqlitePool,
    job_id: &str,
    error: &str,
) -> Result<(), sqlx::Error> {
    let now = models::now_iso();
    sqlx::query(
        "UPDATE jobs SET status = 'failed', last_error = ?, updated_at = ? WHERE id = ?",
    )
    .bind(error)
    .bind(&now)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn reschedule_job(
    pool: &SqlitePool,
    job_id: &str,
    error: &str,
    backoff_seconds: i64,
) -> Result<(), sqlx::Error> {
    let scheduled_at = (chrono::Utc::now() + chrono::Duration::seconds(backoff_seconds))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let now = models::now_iso();
    sqlx::query(
        "UPDATE jobs SET status = 'pending', locked_at = NULL, worker_id = NULL, last_error = ?, scheduled_at = ?, updated_at = ? WHERE id = ?"
    )
    .bind(error)
    .bind(&scheduled_at)
    .bind(&now)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}
