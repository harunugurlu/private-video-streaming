mod db;
mod errors;
mod handlers;
mod models;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pool = db::init_pool("sqlite://data.db").await;

    // Ensure storage directories exist
    for dir in &[
        "storage/originals",
        "storage/hls/_tmp",
        "storage/hls/public",
    ] {
        tokio::fs::create_dir_all(dir)
            .await
            .expect("Failed to create storage directory");
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/videos", post(handlers::create_video))
        .route(
            "/api/videos/{video_id}/source",
            put(handlers::upload_source),
        )
        .route(
            "/api/videos/{video_id}/status",
            get(handlers::get_video_status),
        )
        .route("/api/share/{token}", get(handlers::get_share))
        .nest_service("/media/hls", ServeDir::new("storage/hls/public"))
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024 + 1024 * 1024)) // 1GB + 1MB overhead for multipart headers
        .layer(cors)
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    tracing::info!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "ok"
}
