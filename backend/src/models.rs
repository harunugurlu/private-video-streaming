use serde::{Deserialize, Serialize};

// --- Constants ---

pub const MAX_UPLOAD_BYTES: i64 = 1_073_741_824; // 1 GB

pub const SUPPORTED_MIME_TYPES: &[&str] = &[
    "video/mp4",
    "video/quicktime",
    "video/webm",
    "video/x-msvideo",
    "video/x-matroska",
];

pub const SUPPORTED_EXTENSIONS: &[&str] = &[".mp4", ".mov", ".webm", ".avi", ".mkv"];

// --- Database row types ---

#[derive(Debug, Clone, sqlx::FromRow)]
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

// --- API request types ---

#[derive(Debug, Deserialize)]
pub struct CreateVideoRequest {
    pub title: String,
    pub filename: String,
    pub size_bytes: i64,
    pub mime_type: String,
}

// --- API response types ---

#[derive(Debug, Serialize)]
pub struct CreateVideoResponse {
    pub video_id: String,
    pub share_url: String,
    pub status: String,
    pub max_upload_bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct UploadSourceResponse {
    pub video_id: String,
    pub status: String,
    pub upload_completed_at: String,
}

#[derive(Debug, Serialize)]
pub struct VideoStatusResponse {
    pub video_id: String,
    pub status: String,
    pub processing_stage: Option<String>,
    pub share_url: String,
    pub updated_at: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlaybackInfo {
    pub hls_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ShareResponse {
    pub video_id: String,
    pub title: String,
    pub status: String,
    pub processing_stage: Option<String>,
    pub playback: PlaybackInfo,
    pub created_at: String,
}

// --- Helpers ---

pub fn generate_video_id() -> String {
    format!("vid_{}", uuid::Uuid::new_v4().simple())
}

pub fn generate_share_token() -> String {
    format!("shr_{}", uuid::Uuid::new_v4().simple())
}

pub fn generate_job_id() -> String {
    format!("job_{}", uuid::Uuid::new_v4().simple())
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn has_supported_extension(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    SUPPORTED_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

pub fn is_supported_mime(mime: &str) -> bool {
    SUPPORTED_MIME_TYPES.contains(&mime)
}
