mod access_event_publisher;

use async_trait::async_trait;
use shortener_core::{AppError, messaging::AccessEvent};

pub use access_event_publisher::AccessEventPublisher;

/// Trait for publishing events.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publish an access event.
    async fn publish(&self, event: AccessEvent) -> Result<(), AppError>;
}
