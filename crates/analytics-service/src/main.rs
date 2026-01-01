mod config;
mod consumer;
mod repository;
mod routes;

use std::sync::Arc;

use axum::{Router, routing::get};
use shortener_core::telemetry;
use tower_http::trace::TraceLayer;
use tracing::info;

use config::Config;
use consumer::{AccessEventConsumer, EventConsumer};
use repository::AnalyticsRepository;

#[derive(Clone)]
pub struct AppState {
    pub analytics_repository: Arc<AnalyticsRepository>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    let _guard = telemetry::init_tracing(&config.observability_config(), "analytics-service")?;

    let redis_config = config.redis_config();
    let redis_client = redis::Client::open(redis_config.url.expose())?;
    let analytics_repository = Arc::new(AnalyticsRepository::new(redis_client));

    let consumer =
        AccessEventConsumer::new(&config.rabbitmq_config(), Arc::clone(&analytics_repository))
            .await?;

    tokio::spawn(async move {
        if let Err(e) = consumer.start_consuming().await {
            tracing::error!("Consumer error: {:?}", e);
        }
    });

    let state = AppState {
        analytics_repository,
    };

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/ready", get(routes::ready))
        .route("/api/v1/analytics", get(routes::list_analytics))
        .route("/api/v1/analytics/{code}", get(routes::get_analytics))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = config.server_addr();
    info!("Starting analytics-service on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
