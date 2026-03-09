use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateVideoRequest {
    pub title: String,
    pub filename: String,
    pub size_bytes: i64,
    pub mime_type: String,
}

#[derive(Debug, Serialize)]
pub struct CreateVideoResponse {
    pub video_id: String,
    pub share_token: String,
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
    pub share_token: String,
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
