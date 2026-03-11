#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct VideoRow {
    pub id: String,
    pub title: String,
    pub filename: String,
    pub size_bytes: i64,
    pub mime_type: String,
    pub status: String,
    pub share_token: String,
    pub processing_stage: Option<String>,
    pub source_path: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub upload_initiated_at: String,
    pub upload_completed_at: Option<String>,
    pub processing_started_at: Option<String>,
    pub processing_completed_at: Option<String>,
    pub failed_at: Option<String>,
    pub failure_stage: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
