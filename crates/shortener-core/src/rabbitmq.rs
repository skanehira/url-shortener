//! `RabbitMQ` connection and channel management.

use lapin::{
    Channel, Connection, ConnectionProperties, ExchangeKind,
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
};
use tracing::{info, instrument};

use crate::{AppError, config::RabbitMQConfig};

/// `RabbitMQChannel` wrapper that handles connection setup.
pub struct RabbitMQChannel {
    pub channel: Channel,
    pub exchange: String,
    pub queue: String,
    pub routing_key: String,
}

impl RabbitMQChannel {
    /// Connect to `RabbitMQ` and set up exchange/queue.
    ///
    /// # Errors
    ///
    /// Returns `AppError::MessageQueue` if connection, channel creation,
    /// exchange/queue declaration, or queue binding fails.
    #[instrument(skip(config))]
    pub async fn try_new(config: &RabbitMQConfig) -> Result<Self, AppError> {
        let conn = Connection::connect(config.url.expose(), ConnectionProperties::default())
            .await
            .map_err(|e| AppError::MessageQueue(e.to_string()))?;

        let channel = conn
            .create_channel()
            .await
            .map_err(|e| AppError::MessageQueue(e.to_string()))?;

        channel
            .exchange_declare(
                &config.exchange,
                ExchangeKind::Topic,
                ExchangeDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::MessageQueue(e.to_string()))?;

        channel
            .queue_declare(
                &config.queue,
                QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::MessageQueue(e.to_string()))?;

        channel
            .queue_bind(
                &config.queue,
                &config.exchange,
                &config.routing_key,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::MessageQueue(e.to_string()))?;

        info!(
            exchange = %config.exchange,
            queue = %config.queue,
            "RabbitMQ channel initialized"
        );

        Ok(Self {
            channel,
            exchange: config.exchange.clone(),
            queue: config.queue.clone(),
            routing_key: config.routing_key.clone(),
        })
    }
}
