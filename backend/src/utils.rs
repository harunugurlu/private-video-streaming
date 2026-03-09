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
