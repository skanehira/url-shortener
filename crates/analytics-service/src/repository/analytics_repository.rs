use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client};
use serde::Serialize;
use shortener_core::AppError;
use tokio::sync::Mutex;
use tracing::instrument;

const KEY_PREFIX_COUNT: &str = "access:count:";
const KEY_PREFIX_LAST: &str = "access:last:";
const KEY_CODES: &str = "access:codes";

#[derive(Debug, Clone, Serialize)]
pub struct Analytics {
    pub code: String,
    pub access_count: i64,
    pub last_accessed_at: Option<DateTime<Utc>>,
}

pub struct AnalyticsRepository {
    client: Client,
    conn: Mutex<Option<redis::aio::MultiplexedConnection>>,
}

impl AnalyticsRepository {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            conn: Mutex::new(None),
        }
    }

    async fn get_conn(&self) -> Result<redis::aio::MultiplexedConnection, AppError> {
        let mut guard = self.conn.lock().await;
        if guard.is_none() {
            let conn = self
                .client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| AppError::Redis(e.to_string()))?;
            *guard = Some(conn);
        }
        Ok(guard.clone().unwrap())
    }

    /// Check Redis connection with PING command.
    #[instrument(skip(self))]
    pub async fn ping(&self) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(|e| AppError::Redis(e.to_string()))?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn increment(&self, code: &str, accessed_at: DateTime<Utc>) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;

        let count_key = format!("{KEY_PREFIX_COUNT}{code}");
        let last_key = format!("{KEY_PREFIX_LAST}{code}");

        redis::pipe()
            .atomic()
            .incr(&count_key, 1i64)
            .set(&last_key, accessed_at.to_rfc3339())
            .sadd(KEY_CODES, code)
            .exec_async(&mut conn)
            .await
            .map_err(|e| AppError::Redis(e.to_string()))?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get(&self, code: &str) -> Result<Option<Analytics>, AppError> {
        let mut conn = self.get_conn().await?;

        let count_key = format!("{KEY_PREFIX_COUNT}{code}");
        let last_key = format!("{KEY_PREFIX_LAST}{code}");

        let count: Option<i64> = conn
            .get(&count_key)
            .await
            .map_err(|e| AppError::Redis(e.to_string()))?;

        let Some(access_count) = count else {
            return Ok(None);
        };

        let last: Option<String> = conn
            .get(&last_key)
            .await
            .map_err(|e| AppError::Redis(e.to_string()))?;

        let last_accessed_at = last
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(Some(Analytics {
            code: code.to_string(),
            access_count,
            last_accessed_at,
        }))
    }

    #[instrument(skip(self))]
    pub async fn list(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<Analytics>, usize), AppError> {
        let mut conn = self.get_conn().await?;

        let codes: Vec<String> = conn
            .smembers(KEY_CODES)
            .await
            .map_err(|e| AppError::Redis(e.to_string()))?;

        let total = codes.len();

        let mut analytics = Vec::new();
        for code in codes.iter().skip(offset).take(limit) {
            if let Some(a) = self.get(code).await? {
                analytics.push(a);
            }
        }

        analytics.sort_by(|a, b| b.access_count.cmp(&a.access_count));

        Ok((analytics, total))
    }
}
