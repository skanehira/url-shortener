mod config;
mod publisher;
mod repository;
mod routes;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Router, routing::get};
use shortener_core::telemetry;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing::info;

use config::Config;
use publisher::{AccessEventPublisher, EventPublisher};
use repository::UrlRepository;

#[derive(Clone)]
pub struct AppState {
    pub url_repository: UrlRepository,
    pub event_publisher: Arc<dyn EventPublisher>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    let _guard = telemetry::init_tracing(&config.observability_config(), "shortener-service")?;

    let db_config = config.database_config();
    let db_pool = PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .connect(db_config.url.expose())
        .await?;

    sqlx::migrate!("./migrations").run(&db_pool).await?;

    let event_publisher: Arc<dyn EventPublisher> =
        Arc::new(AccessEventPublisher::new(&config.rabbitmq_config()).await?);

    let state = AppState {
        url_repository: UrlRepository::new(db_pool),
        event_publisher,
    };

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/ready", get(routes::ready))
        .route(
            "/api/v1/urls",
            get(routes::list_urls).post(routes::create_url),
        )
        .route(
            "/api/v1/urls/{code}",
            get(routes::get_url)
                .put(routes::update_url)
                .delete(routes::delete_url),
        )
        .route("/{code}", get(routes::redirect))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = config.server_addr();
    info!("Starting shortener-service on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
