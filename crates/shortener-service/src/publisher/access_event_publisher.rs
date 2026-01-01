use std::collections::HashMap;

use async_trait::async_trait;
use lapin::{
    BasicProperties,
    options::BasicPublishOptions,
    types::{AMQPValue, FieldTable, LongString},
};
use opentelemetry::{global, propagation::Injector};
use shortener_core::{AppError, RabbitMQChannel, config::RabbitMQConfig, messaging::AccessEvent};
use tracing::{Span, info, instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::EventPublisher;

pub struct AccessEventPublisher {
    rabbitmq: RabbitMQChannel,
}

/// Carrier for injecting trace context into message headers.
struct HeaderInjector(HashMap<String, String>);

impl Injector for HeaderInjector {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

impl AccessEventPublisher {
    #[instrument(skip(config))]
    pub async fn new(config: &RabbitMQConfig) -> Result<Self, AppError> {
        let rabbitmq = RabbitMQChannel::new(config).await?;
        Ok(Self { rabbitmq })
    }
}

#[async_trait]
impl EventPublisher for AccessEventPublisher {
    #[instrument(skip(self))]
    async fn publish(&self, event: AccessEvent) -> Result<(), AppError> {
        let payload =
            serde_json::to_vec(&event).map_err(|e| AppError::Serialization(e.to_string()))?;

        // Inject trace context into headers
        let mut injector = HeaderInjector(HashMap::new());
        let context = Span::current().context();
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&context, &mut injector);
        });

        // Convert to FieldTable for RabbitMQ headers
        let mut headers = FieldTable::default();
        for (k, v) in injector.0 {
            headers.insert(k.into(), AMQPValue::LongString(LongString::from(v)));
        }

        let properties = BasicProperties::default()
            .with_content_type("application/json".into())
            .with_delivery_mode(2)
            .with_message_id(event.event_id.to_string().into())
            .with_headers(headers);

        self.rabbitmq
            .channel
            .basic_publish(
                &self.rabbitmq.exchange,
                &self.rabbitmq.routing_key,
                BasicPublishOptions::default(),
                &payload,
                properties,
            )
            .await
            .map_err(|e| AppError::MessageQueue(e.to_string()))?;

        info!(
            event_id = %event.event_id,
            code = %event.code,
            "Access event published"
        );

        Ok(())
    }
}
