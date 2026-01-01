//! Configuration for shortener-service.

use saferet::SecretString;
use serviceconf::ServiceConf;
use shortener_core::config::{DatabaseConfig, ObservabilityConfig, RabbitMQConfig};

/// Configuration for shortener-service.
#[derive(Debug, Clone, ServiceConf)]
pub struct Config {
    /// Database connection URL.
    #[conf(from_file)]
    pub database_url: SecretString,

    /// Maximum number of database connections.
    #[conf(default = 10)]
    pub database_max_connections: u32,

    /// `RabbitMQ` connection URL.
    #[conf(from_file)]
    pub rabbitmq_url: SecretString,

    /// `RabbitMQ` exchange name.
    #[conf(default = "url_shortener".to_string())]
    pub rabbitmq_exchange: String,

    /// `RabbitMQ` queue name.
    #[conf(default = "access_events".to_string())]
    pub rabbitmq_queue: String,

    /// `RabbitMQ` routing key.
    #[conf(default = "access.event".to_string())]
    pub rabbitmq_routing_key: String,

    /// OTEL exporter endpoint (optional).
    #[conf(from_file)]
    pub otel_exporter_endpoint: Option<SecretString>,

    /// Server host address.
    #[conf(default = "0.0.0.0".to_string())]
    pub server_host: String,

    /// Server port.
    #[conf(default = 8080)]
    pub server_port: u16,
}

impl Config {
    /// Returns the database configuration.
    #[must_use]
    pub fn database_config(&self) -> DatabaseConfig {
        DatabaseConfig {
            url: self.database_url.clone(),
            max_connections: self.database_max_connections,
        }
    }

    /// Returns the `RabbitMQ` configuration.
    #[must_use]
    pub fn rabbitmq_config(&self) -> RabbitMQConfig {
        RabbitMQConfig {
            url: self.rabbitmq_url.clone(),
            exchange: self.rabbitmq_exchange.clone(),
            queue: self.rabbitmq_queue.clone(),
            routing_key: self.rabbitmq_routing_key.clone(),
        }
    }

    /// Returns the observability configuration.
    #[must_use]
    pub fn observability_config(&self) -> ObservabilityConfig {
        ObservabilityConfig {
            otlp_endpoint: self.otel_exporter_endpoint.clone(),
        }
    }

    /// Returns the server address.
    #[must_use]
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}
