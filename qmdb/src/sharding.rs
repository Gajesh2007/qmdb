use std::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;
use crate::def::SHARD_COUNT;

/// The current shard count used by the system.
/// Fixed at SHARD_COUNT (16) based on extensive benchmarking that showed:
/// - 16 shards provides optimal performance in single-node deployments
/// - Performance was significantly worse with 4 shards (under-parallelized)
/// - Performance degraded with 32 shards (overhead of coordination)
/// This value is intentionally not configurable to prevent suboptimal deployments.
lazy_static! {
    static ref CURRENT_SHARD_COUNT: AtomicUsize = AtomicUsize::new(SHARD_COUNT);
}

/// Gets the current shard count.
/// Always returns SHARD_COUNT (16) as this has been proven optimal through benchmarking.
pub fn get_current_shard_count() -> usize {
    CURRENT_SHARD_COUNT.load(Ordering::SeqCst)
}

/// Gets the current sentry count based on the shard count.
/// With SHARD_COUNT=16, this ensures optimal distribution of sentries.
pub fn get_current_sentry_count() -> usize {
    (1 << 16) / get_current_shard_count()
}

/// Gets the current shard division factor based on the shard count.
/// With SHARD_COUNT=16, this provides the optimal division of the key space.
pub fn get_current_shard_div() -> usize {
    (1 << 16) / get_current_shard_count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::def::{SENTRY_COUNT, SHARD_DIV};

    #[test]
    fn test_shard_count() {
        assert_eq!(get_current_shard_count(), 16);
        assert_eq!(get_current_sentry_count(), SENTRY_COUNT);
        assert_eq!(get_current_shard_div(), SHARD_DIV);
    }
} 