use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Redirect},
};
use shortener_core::{AppError, messaging::AccessEvent};
use tracing::{Instrument, Span, info, info_span, instrument};

use crate::AppState;

#[instrument(skip(state, headers))]
pub async fn redirect(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let url = state
        .url_repository
        .find_by_code(&code)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("URL with code '{code}' not found")))?;

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let referer = headers
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let publisher = state.event_publisher.clone();
    let code_clone = code.clone();
    let ip = addr.ip().to_string();

    // Create a span for the background task that follows from the current span
    let current_span = Span::current();
    let span = info_span!(parent: &current_span, "publish_access_event", code = %code_clone);

    tokio::spawn(
        async move {
            let event = AccessEvent::new(code_clone, user_agent, Some(ip), referer);
            if let Err(e) = publisher.publish(event).await {
                tracing::warn!("Failed to publish access event: {:?}", e);
            }
        }
        .instrument(span),
    );

    info!(code = %code, original_url = %url.original_url, "Redirecting");

    Ok(Redirect::temporary(&url.original_url))
}
