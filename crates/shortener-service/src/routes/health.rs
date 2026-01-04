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
    database: Option<&'static str>,
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
    // Check database connection
    let db_ok = sqlx::query!(r#"SELECT 1 as "one!""#)
        .fetch_one(state.url_repository.pool())
        .await
        .is_ok();

    if db_ok {
        (
            StatusCode::OK,
            Json(ReadyResponse {
                status: "ready",
                database: Some("ok"),
            }),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse {
                status: "not ready",
                database: Some("unavailable"),
            }),
        )
    }
}
