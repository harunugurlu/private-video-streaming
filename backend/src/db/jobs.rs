use sqlx::SqlitePool;

use crate::models::JobRow;
use crate::utils;

pub async fn insert(
    pool: &SqlitePool,
    video_id: &str,
    source_path: &str,
) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
    let job_id = utils::generate_job_id();
    let idempotency_key = format!("process_video:{}", video_id);

    sqlx::query(
        "INSERT INTO jobs (id, type, video_id, source_path, status, scheduled_at, idempotency_key, created_at, updated_at)
         VALUES (?, 'process_video', ?, ?, 'pending', ?, ?, ?, ?)
         ON CONFLICT(idempotency_key) DO NOTHING",
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

pub async fn claim_pending(
    pool: &SqlitePool,
    worker_id: &str,
) -> Result<Option<JobRow>, sqlx::Error> {
    let now = utils::now_iso();

    sqlx::query_as::<_, JobRow>(
        "UPDATE jobs SET status = 'in_progress', locked_at = ?, worker_id = ?, attempt_count = attempt_count + 1, updated_at = ?
         WHERE id = (SELECT id FROM jobs WHERE status = 'pending' ORDER BY scheduled_at ASC LIMIT 1)
         RETURNING *",
    )
    .bind(&now)
    .bind(worker_id)
    .bind(&now)
    .fetch_optional(pool)
    .await
}

pub async fn mark_done(pool: &SqlitePool, job_id: &str) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
    sqlx::query("UPDATE jobs SET status = 'done', updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(job_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn mark_failed(
    pool: &SqlitePool,
    job_id: &str,
    error: &str,
) -> Result<(), sqlx::Error> {
    let now = utils::now_iso();
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

pub async fn reschedule(
    pool: &SqlitePool,
    job_id: &str,
    error: &str,
    backoff_seconds: i64,
) -> Result<(), sqlx::Error> {
    let scheduled_at = (chrono::Utc::now() + chrono::Duration::seconds(backoff_seconds))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let now = utils::now_iso();
    sqlx::query(
        "UPDATE jobs SET status = 'pending', locked_at = NULL, worker_id = NULL, last_error = ?, scheduled_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(error)
    .bind(&scheduled_at)
    .bind(&now)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}
