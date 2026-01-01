//! Shared configuration types for all services.

pub use saferet::SecretString;
pub use serviceconf::ServiceConf;

/// Database configuration.
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: SecretString,
    pub max_connections: u32,
}

/// Redis configuration.
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: SecretString,
}

/// `RabbitMQ` configuration.
#[derive(Debug, Clone)]
pub struct RabbitMQConfig {
    pub url: SecretString,
    pub exchange: String,
    pub queue: String,
    pub routing_key: String,
}

/// Observability configuration.
#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    pub otlp_endpoint: Option<SecretString>,
}
