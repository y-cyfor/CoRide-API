use std::time::Duration;

use chrono::Utc;
use sqlx::SqlitePool;

use crate::config::HealthCheckConfig;
use crate::db::models::maybe_decrypt_api_keys;
use crate::utils::encrypt::KeyRing;

/// Outcome of one health-check sweep.
#[derive(Debug, Default)]
pub struct HealthReport {
    pub checked: usize,
    pub disabled: Vec<i64>,
    pub recovered: Vec<i64>,
    pub failed: Vec<i64>,
    /// Channels that could not be probed because their configuration is unusable
    /// (e.g. undecryptable API keys). These are reported but never auto-disabled.
    pub skipped: Vec<i64>,
}

struct ProbeTarget {
    id: i64,
    name: String,
    channel_type: String,
    base_url: String,
    api_keys: String,
    proxy_url: Option<String>,
    status: String,
    consecutive_failures: i32,
    auto_disabled: bool,
    timeout_ms: i32,
}

/// Check the health of every channel that is active or was auto-disabled.
///
/// Channels disabled by an administrator (`auto_disabled = 0`) are left untouched, and
/// channels that this checker disabled are re-probed so they can recover automatically.
pub async fn check_channels_health(
    pool: &SqlitePool,
    ring: &KeyRing,
    cfg: &HealthCheckConfig,
) -> Result<HealthReport, Box<dyn std::error::Error + Send + Sync>> {
    type ChannelHealthRow = (
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        i32,
        bool,
        i32,
    );
    let rows: Vec<ChannelHealthRow> = sqlx::query_as(
        "SELECT id, name, type, base_url, api_keys, proxy_url, status, consecutive_failures, auto_disabled, timeout
         FROM channels
         WHERE status = 'active' OR (status = 'disabled' AND auto_disabled = 1)",
    )
    .fetch_all(pool)
    .await?;

    let targets: Vec<ProbeTarget> = rows
        .into_iter()
        .map(
            |(
                id,
                name,
                channel_type,
                base_url,
                api_keys,
                proxy_url,
                status,
                consecutive_failures,
                auto_disabled,
                timeout_ms,
            )| ProbeTarget {
                id,
                name,
                channel_type,
                base_url,
                api_keys,
                proxy_url,
                status,
                consecutive_failures,
                auto_disabled,
                timeout_ms,
            },
        )
        .collect();

    let mut report = HealthReport::default();
    if targets.is_empty() {
        return Ok(report);
    }

    let threshold = cfg.failure_threshold.max(1);

    for target in targets {
        report.checked += 1;

        // Decrypt before probing: channels are stored encrypted, and sending the
        // ciphertext upstream used to disable every channel after three sweeps.
        let keys: Vec<String> = match serde_json::from_str(&maybe_decrypt_api_keys(
            &target.api_keys,
            ring,
        )) {
            Ok(keys) => keys,
            Err(e) => {
                tracing::warn!(
                    channel_id = target.id,
                    channel = %target.name,
                    error = %e,
                    "Skipping health check: API keys could not be parsed"
                );
                report.skipped.push(target.id);
                continue;
            }
        };

        let Some(api_key) = keys.first() else {
            tracing::warn!(
                channel_id = target.id,
                channel = %target.name,
                "Skipping health check: no API key configured"
            );
            report.skipped.push(target.id);
            continue;
        };

        let timeout_secs = if target.timeout_ms > 0 {
            (target.timeout_ms as u64 / 1000).clamp(3, 120)
        } else {
            cfg.timeout_secs.clamp(3, 120)
        };

        let client = build_probe_client(target.proxy_url.as_deref(), timeout_secs);
        let test_url = format!("{}/models", target.base_url.trim_end_matches('/'));

        let mut req = client.get(&test_url);
        if target.channel_type == "anthropic" {
            req = req
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01");
        } else {
            req = req.header("Authorization", format!("Bearer {api_key}"));
        }

        let now = Utc::now();
        let outcome = classify_probe(req.send().await);

        match outcome {
            ProbeOutcome::Healthy => {
                sqlx::query(
                    "UPDATE channels
                     SET consecutive_failures = 0, auto_disabled = 0,
                         status = CASE WHEN auto_disabled = 1 THEN 'active' ELSE status END,
                         last_checked = ?, last_success_at = ?, updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?",
                )
                .bind(now)
                .bind(now)
                .bind(target.id)
                .execute(pool)
                .await?;

                if target.status == "disabled" && target.auto_disabled {
                    report.recovered.push(target.id);
                    tracing::info!(
                        channel_id = target.id,
                        channel = %target.name,
                        "Channel recovered, re-enabling"
                    );
                }
            }
            ProbeOutcome::Inconclusive(reason) => {
                // The endpoint is missing or the provider throttled us: not evidence
                // that the channel is broken.
                tracing::debug!(
                    channel_id = target.id,
                    channel = %target.name,
                    reason = %reason,
                    "Health check inconclusive, keeping current state"
                );
                sqlx::query("UPDATE channels SET last_checked = ? WHERE id = ?")
                    .bind(now)
                    .bind(target.id)
                    .execute(pool)
                    .await?;
            }
            ProbeOutcome::Failed(reason) => {
                let failures = target.consecutive_failures + 1;
                report.failed.push(target.id);

                if failures >= threshold {
                    sqlx::query(
                        "UPDATE channels
                         SET status = 'disabled', auto_disabled = 1, consecutive_failures = ?,
                             last_checked = ?, updated_at = CURRENT_TIMESTAMP
                         WHERE id = ?",
                    )
                    .bind(failures)
                    .bind(now)
                    .bind(target.id)
                    .execute(pool)
                    .await?;
                    report.disabled.push(target.id);
                    tracing::warn!(
                        channel_id = target.id,
                        channel = %target.name,
                        failures,
                        reason = %reason,
                        "Channel disabled by health check (will be re-probed for recovery)"
                    );
                } else {
                    sqlx::query(
                        "UPDATE channels SET consecutive_failures = ?, last_checked = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                    )
                    .bind(failures)
                    .bind(now)
                    .bind(target.id)
                    .execute(pool)
                    .await?;
                    tracing::warn!(
                        channel_id = target.id,
                        channel = %target.name,
                        failures,
                        reason = %reason,
                        "Channel health check failed"
                    );
                }
            }
        }
    }

    Ok(report)
}

enum ProbeOutcome {
    Healthy,
    /// Could not determine health; keep the current state.
    Inconclusive(String),
    Failed(String),
}

fn classify_probe(result: Result<reqwest::Response, reqwest::Error>) -> ProbeOutcome {
    match result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                ProbeOutcome::Healthy
            } else if status == reqwest::StatusCode::NOT_FOUND
                || status == reqwest::StatusCode::METHOD_NOT_ALLOWED
            {
                // Provider does not implement /models; connectivity is fine.
                ProbeOutcome::Inconclusive(format!("/models unsupported (HTTP {})", status.as_u16()))
            } else if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                ProbeOutcome::Inconclusive("rate limited by provider".to_string())
            } else {
                ProbeOutcome::Failed(format!("HTTP {}", status.as_u16()))
            }
        }
        Err(e) => {
            if e.is_timeout() {
                ProbeOutcome::Failed("timeout".to_string())
            } else {
                ProbeOutcome::Failed(e.to_string())
            }
        }
    }
}

fn build_probe_client(proxy_url: Option<&str>, timeout_secs: u64) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(10))
        .pool_idle_timeout(Duration::from_secs(60));
    if let Some(url) = proxy_url
        && let Ok(proxy) = reqwest::Proxy::all(url) {
            builder = builder.proxy(proxy);
        }
    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_defaults_to_empty() {
        let report = HealthReport::default();
        assert_eq!(report.checked, 0);
        assert!(report.disabled.is_empty());
        assert!(report.skipped.is_empty());
    }

    #[test]
    fn probe_timeout_is_clamped_to_a_sane_window() {
        // A channel configured with a 1 ms timeout must still allow a probe.
        let secs = |ms: i32| -> u64 {
            if ms > 0 {
                (ms as u64 / 1000).clamp(3, 120)
            } else {
                30
            }
        };
        assert_eq!(secs(1), 3);
        assert_eq!(secs(300_000), 120);
        assert_eq!(secs(0), 30);
    }
}

