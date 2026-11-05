use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use governor::clock::DefaultClock;
use governor::middleware::NoOpMiddleware;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorLimiter};

pub type DirectLimiter = GovernorLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

/// Outcome of a concurrency-limit acquisition.
pub enum ConcurrencyOutcome {
    /// No limit configured: nothing was reserved.
    Unlimited,
    /// A slot was reserved; released automatically on drop.
    Acquired(ConcurrencyGuard),
    /// The limit is reached and the caller must reject/queue.
    Denied,
}

/// RAII guard for a concurrency slot.
///
/// The slot is returned when the guard is dropped, which makes cancellation and error
/// paths safe: a request that is aborted never leaks a slot.
pub struct ConcurrencyGuard {
    map: Arc<DashMap<String, Arc<AtomicU32>>>,
    key: String,
}

impl Drop for ConcurrencyGuard {
    fn drop(&mut self) {
        if let Some(counter) = self.map.get(&self.key) {
            // Saturating: never wrap below zero.
            let _ = counter.fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                Some(value.saturating_sub(1))
            });
        }
    }
}

/// Registry of rate limiters and concurrency counters keyed by an arbitrary string.
///
/// Keys are namespaced by the caller, e.g. `global`, `user:12`, `channel:3`.
#[derive(Default)]
pub struct Limiters {
    qps: DashMap<String, (u32, Arc<DirectLimiter>)>,
    concurrent: Arc<DashMap<String, Arc<AtomicU32>>>,
}

impl Limiters {
    pub fn new() -> Self {
        Self::default()
    }

    fn limiter_for(&self, key: &str, qps: u32) -> Option<Arc<DirectLimiter>> {
        if qps == 0 {
            return None;
        }
        let limit = NonZeroU32::new(qps).or_else(|| NonZeroU32::new(1))?;

        if let Some(entry) = self.qps.get(key) {
            let (stored_qps, limiter) = entry.value();
            if *stored_qps == qps {
                return Some(limiter.clone());
            }
        }

        let limiter = Arc::new(GovernorLimiter::direct(Quota::per_second(limit)));
        self.qps.insert(key.to_string(), (qps, limiter.clone()));
        Some(limiter)
    }

    /// Non-blocking QPS check. Returns true when the request may proceed.
    pub fn try_acquire_qps(&self, key: &str, qps: u32) -> bool {
        match self.limiter_for(key, qps) {
            Some(limiter) => limiter.check().is_ok(),
            None => true,
        }
    }

    /// Blocking QPS check used when the configured action is `queue`.
    ///
    /// Waiting is bounded by `timeout` so a saturated limiter cannot hang a request.
    pub async fn acquire_qps_wait(&self, key: &str, qps: u32, timeout: Duration) -> bool {
        let Some(limiter) = self.limiter_for(key, qps) else {
            return true;
        };
        if limiter.check().is_ok() {
            return true;
        }
        tokio::time::timeout(timeout, limiter.until_ready())
            .await
            .is_ok()
    }

    /// Reserve a concurrency slot.
    pub fn try_acquire_concurrency(&self, key: &str, max: u32) -> ConcurrencyOutcome {
        if max == 0 {
            return ConcurrencyOutcome::Unlimited;
        }

        let counter = self
            .concurrent
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(AtomicU32::new(0)))
            .clone();

        loop {
            let current = counter.load(Ordering::Acquire);
            if current >= max {
                return ConcurrencyOutcome::Denied;
            }
            if counter
                .compare_exchange_weak(current, current + 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return ConcurrencyOutcome::Acquired(ConcurrencyGuard {
                    map: Arc::clone(&self.concurrent),
                    key: key.to_string(),
                });
            }
        }
    }

    /// Reserve a concurrency slot, waiting up to `timeout` for one to free up.
    ///
    /// Returns `None` when the limit stayed saturated for the whole timeout.
    pub async fn acquire_concurrency_wait(
        &self,
        key: &str,
        max: u32,
        timeout: Duration,
    ) -> Option<ConcurrencyGuard> {
        if max == 0 {
            return None;
        }

        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if let ConcurrencyOutcome::Acquired(guard) = self.try_acquire_concurrency(key, max) {
                return Some(guard);
            }
            if tokio::time::Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    /// Current value of a concurrency counter (used by tests and diagnostics).
    pub fn concurrency_in_use(&self, key: &str) -> u32 {
        self.concurrent
            .get(key)
            .map(|counter| counter.load(Ordering::Acquire))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qps_limiter_rejects_burst_beyond_rate() {
        let limiters = Limiters::new();
        assert!(limiters.try_acquire_qps("t", 2));
        assert!(limiters.try_acquire_qps("t", 2));
        assert!(!limiters.try_acquire_qps("t", 2), "third call must be limited");
    }

    #[test]
    fn qps_zero_means_unlimited() {
        let limiters = Limiters::new();
        for _ in 0..100 {
            assert!(limiters.try_acquire_qps("t", 0));
        }
    }

    #[test]
    fn concurrency_guard_releases_on_drop() {
        let limiters = Limiters::new();
        {
            let outcome = limiters.try_acquire_concurrency("g", 1);
            assert!(matches!(outcome, ConcurrencyOutcome::Acquired(_)));
            assert_eq!(limiters.concurrency_in_use("g"), 1);
            assert!(matches!(
                limiters.try_acquire_concurrency("g", 1),
                ConcurrencyOutcome::Denied
            ));
        }
        assert_eq!(limiters.concurrency_in_use("g"), 0);
        assert!(matches!(
            limiters.try_acquire_concurrency("g", 1),
            ConcurrencyOutcome::Acquired(_)
        ));
    }

    #[test]
    fn concurrency_zero_means_unlimited() {
        let limiters = Limiters::new();
        assert!(matches!(
            limiters.try_acquire_concurrency("g", 0),
            ConcurrencyOutcome::Unlimited
        ));
        assert_eq!(limiters.concurrency_in_use("g"), 0);
    }

    #[test]
    fn limiters_are_independent_per_key() {
        let limiters = Limiters::new();
        assert!(limiters.try_acquire_qps("a", 1));
        assert!(!limiters.try_acquire_qps("a", 1));
        assert!(limiters.try_acquire_qps("b", 1));
    }
}
