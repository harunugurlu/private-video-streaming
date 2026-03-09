use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use sqlx::SqlitePool;

use crate::dto::{CreateVideoRequest, CreateVideoResponse, ShareResponse, UploadSourceResponse, VideoStatusResponse};
use crate::errors::AppError;
use crate::service;

pub async fn create_video(
    State(pool): State<SqlitePool>,
    Json(body): Json<CreateVideoRequest>,
) -> Result<(StatusCode, Json<CreateVideoResponse>), AppError> {
    let response = service::create_video(&pool, body).await?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn upload_source(
    State(pool): State<SqlitePool>,
    Path(video_id): Path<String>,
    multipart: Multipart,
) -> Result<Json<UploadSourceResponse>, AppError> {
    let response = service::upload_source(&pool, &video_id, multipart).await?;
    Ok(Json(response))
}

pub async fn get_video_status(
    State(pool): State<SqlitePool>,
    Path(video_id): Path<String>,
) -> Result<Json<VideoStatusResponse>, AppError> {
    let response = service::get_video_status(&pool, &video_id).await?;
    Ok(Json(response))
}

pub async fn get_share(
    State(pool): State<SqlitePool>,
    Path(token): Path<String>,
) -> Result<Json<ShareResponse>, AppError> {
    let response = service::get_share(&pool, &token).await?;
    Ok(Json(response))
}
