pub const MAX_UPLOAD_BYTES: i64 = 1_073_741_824; // 1 GB

pub const SUPPORTED_MIME_TYPES: &[&str] = &[
    "video/mp4",
    "video/quicktime",
    "video/webm",
    "video/x-msvideo",
    "video/x-matroska",
];

pub const SUPPORTED_EXTENSIONS: &[&str] = &[".mp4", ".mov", ".webm", ".avi", ".mkv"];

pub fn has_supported_extension(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    SUPPORTED_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

pub fn is_supported_mime(mime: &str) -> bool {
    SUPPORTED_MIME_TYPES.contains(&mime)
}
