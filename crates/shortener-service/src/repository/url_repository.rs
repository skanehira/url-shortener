use chrono::{DateTime, Utc};
use rand::Rng;
use serde::Serialize;
use shortener_core::AppError;
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

const CODE_CHARSET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const CODE_LENGTH: usize = 6;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[allow(clippy::struct_field_names)]
pub struct Url {
    pub id: Uuid,
    pub code: String,
    pub original_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Clone)]
pub struct UrlRepository {
    pool: PgPool,
}

impl UrlRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    fn generate_code() -> String {
        let mut rng = rand::thread_rng();
        (0..CODE_LENGTH)
            .map(|_| {
                let idx = rng.gen_range(0..CODE_CHARSET.len());
                CODE_CHARSET[idx] as char
            })
            .collect()
    }

    #[instrument(skip(self))]
    pub async fn create(&self, original_url: &str) -> Result<Url, AppError> {
        let code = Self::generate_code();

        let url = sqlx::query_as::<_, Url>(
            r"
            INSERT INTO urls (code, original_url)
            VALUES ($1, $2)
            RETURNING id, code, original_url, created_at, updated_at, expires_at, is_active
            ",
        )
        .bind(&code)
        .bind(original_url)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(url)
    }

    #[instrument(skip(self))]
    pub async fn find_by_code(&self, code: &str) -> Result<Option<Url>, AppError> {
        let url = sqlx::query_as::<_, Url>(
            r"
            SELECT id, code, original_url, created_at, updated_at, expires_at, is_active
            FROM urls
            WHERE code = $1 AND is_active = true
            ",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(url)
    }

    #[instrument(skip(self))]
    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Url>, AppError> {
        let urls = sqlx::query_as::<_, Url>(
            r"
            SELECT id, code, original_url, created_at, updated_at, expires_at, is_active
            FROM urls
            WHERE is_active = true
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            ",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(urls)
    }

    #[instrument(skip(self))]
    pub async fn update(&self, code: &str, original_url: &str) -> Result<Url, AppError> {
        let url = sqlx::query_as::<_, Url>(
            r"
            UPDATE urls
            SET original_url = $2, updated_at = NOW()
            WHERE code = $1 AND is_active = true
            RETURNING id, code, original_url, created_at, updated_at, expires_at, is_active
            ",
        )
        .bind(code)
        .bind(original_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("URL with code '{code}' not found")))?;

        Ok(url)
    }

    #[instrument(skip(self))]
    pub async fn delete(&self, code: &str) -> Result<(), AppError> {
        let result = sqlx::query(
            r"
            UPDATE urls
            SET is_active = false, updated_at = NOW()
            WHERE code = $1 AND is_active = true
            ",
        )
        .bind(code)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "URL with code '{code}' not found"
            )));
        }

        Ok(())
    }
}
