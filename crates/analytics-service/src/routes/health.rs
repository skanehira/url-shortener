use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use tracing::instrument;

use crate::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    service_name: &'static str,
}

#[derive(Serialize)]
pub struct ReadyResponse {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    redis: Option<&'static str>,
}

/// Liveness probe - プロセスが生きているか確認
#[instrument]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service_name: env!("CARGO_PKG_NAME"),
    })
}

/// Readiness probe - トラフィックを受け入れられるか確認
#[instrument(skip(state))]
pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<ReadyResponse>) {
    // Check Redis connection
    let redis_ok = state.analytics_repository.ping().await.is_ok();

    if redis_ok {
        (
            StatusCode::OK,
            Json(ReadyResponse {
                status: "ready",
                redis: Some("ok"),
            }),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse {
                status: "not ready",
                redis: Some("unavailable"),
            }),
        )
    }
}
