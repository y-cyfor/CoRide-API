use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// SplitMix64 PRNG: tiny, fast, good enough for load-spreading decisions.
///
/// This is *not* used for anything security sensitive; secrets use `getrandom`.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Seed from the wall clock, the process id and a per-call counter so that
    /// concurrent requests do not share a stream of identical values.
    pub fn from_entropy() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let seed = now
            .as_secs()
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (now.subsec_nanos() as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
            ^ (std::process::id() as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
            ^ counter.wrapping_mul(0xD6E8_FEB8_6659_FD93);
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform value in `[0, bound)`; returns 0 when `bound` is 0.
    pub fn below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            0
        } else {
            self.next_u64() % bound
        }
    }

    /// Uniform delay in `[min, max]` milliseconds, robust to inverted ranges.
    pub fn delay_ms(&mut self, min: u64, max: u64) -> u64 {
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        min + self.below(max - min + 1)
    }
}

/// Weighted sampling without replacement.
///
/// `weights` are relative weights (they do not have to sum to 100). Items with
/// non-positive weights are placed at the end in their original order, so that a
/// mis-configured channel is still tried as a last resort instead of being dropped.
pub fn weighted_order(weights: &[i64], rng: &mut Rng) -> Vec<usize> {
    let mut remaining: Vec<(usize, i64)> = weights.iter().copied().enumerate().collect();
    let mut order = Vec::with_capacity(remaining.len());

    while remaining.len() > 1 {
        let total: i128 = remaining
            .iter()
            .map(|(_, w)| i128::from((*w).max(0)))
            .sum();
        if total <= 0 {
            break;
        }

        let mut pick = rng.below(total as u64) as i128;
        let mut chosen = None;
        for (idx, (_, weight)) in remaining.iter().enumerate() {
            let weight = i128::from((*weight).max(0));
            if weight == 0 {
                continue;
            }
            if pick < weight {
                chosen = Some(idx);
                break;
            }
            pick -= weight;
        }

        match chosen {
            Some(idx) => order.push(remaining.remove(idx).0),
            None => break,
        }
    }

    order.extend(remaining.into_iter().map(|(idx, _)| idx));
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn rng_is_deterministic_for_a_seed() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..16 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn rng_produces_varied_values() {
        let mut rng = Rng::from_entropy();
        let values: Vec<u64> = (0..32).map(|_| rng.next_u64()).collect();
        let unique: std::collections::HashSet<_> = values.iter().collect();
        assert_eq!(unique.len(), values.len());
    }

    #[test]
    fn below_stays_in_range_and_zero_bound_is_safe() {
        let mut rng = Rng::new(7);
        for _ in 0..1_000 {
            assert!(rng.below(10) < 10);
        }
        assert_eq!(rng.below(0), 0);
    }

    #[test]
    fn delay_is_within_bounds() {
        let mut rng = Rng::new(11);
        for _ in 0..1_000 {
            let d = rng.delay_ms(100, 200);
            assert!((100..=200).contains(&d));
        }
        // Inverted range must not panic or overflow.
        let d = rng.delay_ms(500, 100);
        assert!((100..=500).contains(&d));
    }

    #[test]
    fn weighted_order_contains_every_item_exactly_once() {
        let mut rng = Rng::new(3);
        for _ in 0..100 {
            let order = weighted_order(&[5, 1, 0, 3], &mut rng);
            let mut sorted = order.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, vec![0, 1, 2, 3]);
        }
    }

    #[test]
    fn weighted_order_prefers_heavier_items() {
        let mut rng = Rng::new(99);
        let mut first_counts: HashMap<usize, usize> = HashMap::new();
        for _ in 0..4_000 {
            let order = weighted_order(&[90, 10], &mut rng);
            *first_counts.entry(order[0]).or_default() += 1;
        }
        let heavy = *first_counts.get(&0).unwrap_or(&0);
        let light = *first_counts.get(&1).unwrap_or(&0);
        assert!(heavy > light * 3, "heavy={heavy} light={light}");
    }

    #[test]
    fn weighted_order_tries_zero_weight_items_last() {
        let mut rng = Rng::new(5);
        for _ in 0..200 {
            let order = weighted_order(&[1, 0], &mut rng);
            assert_eq!(order, vec![0, 1]);
        }
    }

    #[test]
    fn weighted_order_handles_all_zero_weights() {
        let mut rng = Rng::new(5);
        assert_eq!(weighted_order(&[0, 0], &mut rng), vec![0, 1]);
        assert!(weighted_order(&[], &mut rng).is_empty());
    }
}
