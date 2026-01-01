use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use futures_lite::stream::StreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions},
    types::{AMQPValue, FieldTable},
};
use opentelemetry::{global, propagation::Extractor, trace::SpanKind};
use shortener_core::{RabbitMQChannel, config::RabbitMQConfig, messaging::AccessEvent};
use tracing::{Instrument, error, info, info_span, instrument, warn};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::EventConsumer;
use crate::repository::AnalyticsRepository;

pub struct AccessEventConsumer {
    rabbitmq: RabbitMQChannel,
    repository: Arc<AnalyticsRepository>,
}

/// Carrier for extracting trace context from message headers.
struct HeaderExtractor(HashMap<String, String>);

impl HeaderExtractor {
    fn from_field_table(headers: &FieldTable) -> Self {
        let map = headers
            .inner()
            .iter()
            .filter_map(|(k, v)| {
                if let AMQPValue::LongString(s) = v {
                    Some((k.to_string(), s.to_string()))
                } else {
                    None
                }
            })
            .collect();
        Self(map)
    }
}

impl Extractor for HeaderExtractor {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }
}

impl AccessEventConsumer {
    #[instrument(skip(config, repository))]
    pub async fn new(
        config: &RabbitMQConfig,
        repository: Arc<AnalyticsRepository>,
    ) -> anyhow::Result<Self> {
        let rabbitmq = RabbitMQChannel::new(config).await?;
        Ok(Self {
            rabbitmq,
            repository,
        })
    }
}

#[async_trait]
impl EventConsumer for AccessEventConsumer {
    async fn start_consuming(&self) -> anyhow::Result<()> {
        let mut consumer = self
            .rabbitmq
            .channel
            .basic_consume(
                &self.rabbitmq.queue,
                "analytics-consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        info!(queue = %self.rabbitmq.queue, "Started consuming access events");

        while let Some(delivery_result) = consumer.next().await {
            match delivery_result {
                Ok(delivery) => {
                    // Extract trace context from headers
                    let parent_context = delivery
                        .properties
                        .headers()
                        .as_ref()
                        .map(|headers| {
                            let extractor = HeaderExtractor::from_field_table(headers);
                            global::get_text_map_propagator(|propagator| {
                                propagator.extract(&extractor)
                            })
                        })
                        .unwrap_or_default();

                    // Create a new span linked to the parent context
                    let span = info_span!(
                        "process_access_event",
                        otel.kind = ?SpanKind::Consumer,
                        messaging.system = "rabbitmq",
                        messaging.destination = %self.rabbitmq.queue,
                    );
                    span.set_parent(parent_context);

                    async {
                        match serde_json::from_slice::<AccessEvent>(&delivery.data) {
                            Ok(event) => {
                                info!(
                                    event_id = %event.event_id,
                                    code = %event.code,
                                    "Processing access event"
                                );

                                if let Err(e) = self
                                    .repository
                                    .increment(&event.code, event.accessed_at)
                                    .await
                                {
                                    error!("Failed to store event: {:?}", e);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to deserialize event: {:?}", e);
                            }
                        }

                        if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                            error!("Failed to ack message: {:?}", e);
                        }
                    }
                    .instrument(span)
                    .await;
                }
                Err(e) => {
                    error!("Consumer error: {:?}", e);
                }
            }
        }

        Ok(())
    }
}
