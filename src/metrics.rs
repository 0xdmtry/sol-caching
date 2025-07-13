use std::time::Duration;
use tracing::info;

pub trait Metrics: Send + Sync {
    fn record_latest_slot(&self, slot: u64);
    fn record_get_blocks_elapsed(&self, elapsed: Duration);
    fn record_is_slot_confirmed_elapsed(&self, elapsed: Duration);
}

pub struct LoggingMetrics;

impl Metrics for LoggingMetrics {
    fn record_latest_slot(&self, slot: u64) {
        info!(target: "metrics", latest_slot = slot, "Recorded latest slot");
    }

    fn record_get_blocks_elapsed(&self, elapsed: Duration) {
        info!(target: "metrics", elapsed_ms = elapsed.as_millis(), "Recorded get_blocks duration");
    }

    fn record_is_slot_confirmed_elapsed(&self, elapsed: Duration) {
        info!(target: "metrics", elapsed_us = elapsed.as_micros(), "Recorded is_slot_confirmed duration");
    }
}
