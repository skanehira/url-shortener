mod access_event_consumer;

use async_trait::async_trait;

pub use access_event_consumer::AccessEventConsumer;

/// Trait for consuming events.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait EventConsumer: Send + Sync {
    /// Start consuming events from the queue.
    async fn start_consuming(&self) -> anyhow::Result<()>;
}
