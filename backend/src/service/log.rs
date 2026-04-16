use sqlx::SqlitePool;

use crate::db::models;

pub async fn log_request(
    pool: &SqlitePool,
    user_api_key: &str,
    channel_id: Option<i64>,
    model: &str,
    endpoint: &str,
    status_code: i32,
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    elapsed_ms: i32,
    request_body: Option<&str>,
    response_body: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), sqlx::Error> {
    let _id = models::insert_request_log(
        pool,
        user_api_key,
        channel_id,
        model,
        endpoint,
        status_code,
        prompt_tokens,
        completion_tokens,
        total_tokens,
        elapsed_ms,
        request_body,
        response_body,
        error_message,
    ).await?;
    Ok(())
}

pub async fn cleanup_old_logs(pool: &SqlitePool, retention_days: i32) -> Result<u64, sqlx::Error> {
    models::cleanup_old_logs(pool, retention_days).await
}
