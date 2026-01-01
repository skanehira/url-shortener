use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use shortener_core::AppError;
use tracing::instrument;

use crate::{AppState, repository::Url};

#[derive(Debug, Deserialize)]
pub struct CreateUrlRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct CreateUrlResponse {
    pub code: String,
    pub short_url: String,
    pub original_url: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUrlRequest {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct ListUrlsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

#[instrument(skip(state))]
pub async fn create_url(
    State(state): State<AppState>,
    Json(req): Json<CreateUrlRequest>,
) -> Result<impl IntoResponse, AppError> {
    url::Url::parse(&req.url).map_err(|e| AppError::UrlParse(e.to_string()))?;

    let url = state.url_repository.create(&req.url).await?;

    let response = CreateUrlResponse {
        short_url: format!("/{}", url.code),
        code: url.code,
        original_url: url.original_url,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[instrument(skip(state))]
pub async fn list_urls(
    State(state): State<AppState>,
    Query(query): Query<ListUrlsQuery>,
) -> Result<Json<Vec<Url>>, AppError> {
    let urls = state.url_repository.list(query.limit, query.offset).await?;
    Ok(Json(urls))
}

#[instrument(skip(state))]
pub async fn get_url(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<Url>, AppError> {
    let url = state
        .url_repository
        .find_by_code(&code)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("URL with code '{code}' not found")))?;
    Ok(Json(url))
}

#[instrument(skip(state))]
pub async fn update_url(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(req): Json<UpdateUrlRequest>,
) -> Result<Json<Url>, AppError> {
    url::Url::parse(&req.url).map_err(|e| AppError::UrlParse(e.to_string()))?;

    let url = state.url_repository.update(&code, &req.url).await?;
    Ok(Json(url))
}

#[instrument(skip(state))]
pub async fn delete_url(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<StatusCode, AppError> {
    state.url_repository.delete(&code).await?;
    Ok(StatusCode::NO_CONTENT)
}
