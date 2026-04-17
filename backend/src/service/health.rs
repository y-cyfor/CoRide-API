use chrono::Utc;
use sqlx::SqlitePool;

/// Check health of all active channels by sending a test request to their /models endpoint.
/// Returns lists of channel IDs that were disabled and recovered.
pub async fn check_channels_health(
    pool: &SqlitePool,
) -> Result<(Vec<i64>, Vec<i64>), Box<dyn std::error::Error + Send + Sync>> {
    // Get all active channels
    let channels: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        "SELECT id, name, type, base_url, api_keys FROM channels WHERE status = 'active'",
    )
    .fetch_all(pool)
    .await?;

    if channels.is_empty() {
        return Ok((vec![], vec![]));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut disabled = vec![];
    let mut recovered = vec![];

    for (id, name, channel_type, base_url, api_keys) in &channels {
        let keys: Vec<String> = match serde_json::from_str(api_keys) {
            Ok(k) => k,
            Err(_) => continue,
        };
        if keys.is_empty() {
            continue;
        }
        let api_key = &keys[0];

        // Get current consecutive_failures
        let failures: i32 = sqlx::query_scalar(
            "SELECT consecutive_failures FROM channels WHERE id = ?",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        let test_url = format!("{}/models", base_url.trim_end_matches('/'));

        let mut req_builder = client.get(&test_url);
        if channel_type == "anthropic" {
            req_builder = req_builder.header("x-api-key", api_key);
        } else {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let now = Utc::now();

        match req_builder.send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    // Channel is responsive - reset failure count
                    if failures > 0 {
                        sqlx::query(
                            "UPDATE channels SET consecutive_failures = 0, last_checked = ?, last_success_at = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                        )
                        .bind(now)
                        .bind(now)
                        .bind(id)
                        .execute(pool)
                        .await?;
                        recovered.push(*id);
                        tracing::info!(channel_id = id, channel_name = name, "Channel health recovered");
                    } else {
                        sqlx::query(
                            "UPDATE channels SET last_checked = ?, last_success_at = ? WHERE id = ?",
                        )
                        .bind(now)
                        .bind(now)
                        .bind(id)
                        .execute(pool)
                        .await?;
                    }
                } else {
                    // Server error (5xx) - increment failure count
                    let new_failures = failures + 1;
                    if new_failures >= 3 {
                        sqlx::query(
                            "UPDATE channels SET status = 'disabled', consecutive_failures = ?, last_checked = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                        )
                        .bind(new_failures)
                        .bind(now)
                        .bind(id)
                        .execute(pool)
                        .await?;
                        disabled.push(*id);
                        tracing::warn!(channel_id = id, channel_name = name, "Channel disabled due to health check failures");
                    } else {
                        sqlx::query(
                            "UPDATE channels SET consecutive_failures = ?, last_checked = ? WHERE id = ?",
                        )
                        .bind(new_failures)
                        .bind(now)
                        .bind(id)
                        .execute(pool)
                        .await?;
                    }
                }
            }
            Err(_) => {
                // Connection failed - increment failure count
                let new_failures = failures + 1;
                if new_failures >= 3 {
                    sqlx::query(
                        "UPDATE channels SET status = 'disabled', consecutive_failures = ?, last_checked = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                    )
                    .bind(new_failures)
                    .bind(now)
                    .bind(id)
                    .execute(pool)
                    .await?;
                    disabled.push(*id);
                    tracing::warn!(channel_id = id, channel_name = name, "Channel disabled due to health check failures");
                } else {
                    sqlx::query(
                        "UPDATE channels SET consecutive_failures = ?, last_checked = ? WHERE id = ?",
                    )
                    .bind(new_failures)
                    .bind(now)
                    .bind(id)
                    .execute(pool)
                    .await?;
                }
            }
        }
    }

    Ok((disabled, recovered))
}
