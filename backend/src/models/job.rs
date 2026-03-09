#[derive(Debug, Clone, sqlx::FromRow)]
pub struct JobRow {
    pub id: String,
    #[sqlx(rename = "type")]
    pub job_type: String,
    pub video_id: String,
    pub source_path: String,
    pub status: String,
    pub attempt_count: i32,
    pub max_attempts: i32,
    pub scheduled_at: String,
    pub locked_at: Option<String>,
    pub worker_id: Option<String>,
    pub last_error: Option<String>,
    pub idempotency_key: String,
    pub created_at: String,
    pub updated_at: String,
}
