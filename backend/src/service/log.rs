use sqlx::SqlitePool;

use crate::db::models::{self, NewRequestLog};

/// Write one request log entry.
#[allow(clippy::too_many_arguments)]
pub async fn log_request(
    pool: &SqlitePool,
    log: &NewRequestLog<'_>,
) -> Result<(), sqlx::Error> {
    models::create_request_log(pool, log).await.map(|_| ())
}

pub async fn cleanup_old_logs(pool: &SqlitePool, retention_days: i32) -> Result<u64, sqlx::Error> {
    models::cleanup_old_logs(pool, retention_days).await
}
