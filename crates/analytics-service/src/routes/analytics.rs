use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use shortener_core::AppError;
use tracing::instrument;

use crate::{AppState, repository::Analytics};

#[derive(Debug, Deserialize)]
pub struct ListAnalyticsQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Serialize)]
pub struct AnalyticsListResponse {
    pub items: Vec<Analytics>,
    pub total: usize,
}

#[instrument(skip(state))]
pub async fn get_analytics(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<Analytics>, AppError> {
    let analytics = state
        .analytics_repository
        .get(&code)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Analytics for code '{code}' not found")))?;

    Ok(Json(analytics))
}

#[instrument(skip(state))]
pub async fn list_analytics(
    State(state): State<AppState>,
    Query(query): Query<ListAnalyticsQuery>,
) -> Result<Json<AnalyticsListResponse>, AppError> {
    let (items, total) = state
        .analytics_repository
        .list(query.limit, query.offset)
        .await?;

    Ok(Json(AnalyticsListResponse { items, total }))
}
