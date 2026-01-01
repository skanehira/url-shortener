use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessEvent {
    pub event_id: Uuid,
    pub code: String,
    pub accessed_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub referer: Option<String>,
}

impl AccessEvent {
    #[must_use]
    pub fn new(
        code: String,
        user_agent: Option<String>,
        ip_address: Option<String>,
        referer: Option<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            code,
            accessed_at: Utc::now(),
            user_agent,
            ip_address,
            referer,
        }
    }
}
