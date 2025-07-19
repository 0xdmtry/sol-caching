use crate::circuit_breaker::CircuitBreaker;
use crate::utils::retry::{with_retry, with_transient_retry};
use crate::{cache::SlotCache, metrics::Metrics, rpc::RpcApi};
use std::{sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tokio::time::{Instant, sleep};
use tracing::{info, warn};

pub fn poll<T: RpcApi + 'static + ?Sized>(
    rpc_client: Arc<T>,
    cache: Arc<SlotCache>,
    metrics: Arc<dyn Metrics + Send + Sync>,
    poll_interval: Duration,
) {
    info!(
        "Starting poller with interval {} seconds",
        poll_interval.as_secs()
    );

    tokio::spawn(async move {
        loop {
            sleep(poll_interval).await;

            info!("cache: {:?}", cache);

            let latest_on_chain = match rpc_client.get_slot().await {
                Ok(slot) => slot,
                Err(e) => {
                    warn!("Failed to get latest slot: {}", e);
                    continue;
                }
            };

            let start_slot = match cache.get_latest_cached_slot().await {
                Some(latest_cached) => latest_cached + 1,
                None => latest_on_chain.saturating_sub(10),
            };

            if start_slot > latest_on_chain {
                info!("Cache is updated. Latest slot: {}", latest_on_chain);
                continue;
            }

            info!(
                "Fetching confirmed blocks from {} to {}",
                start_slot, latest_on_chain
            );

            let now = Instant::now();
            let blocks_result = rpc_client
                .get_blocks(start_slot, Some(latest_on_chain))
                .await;

            metrics.record_get_blocks_elapsed(now.elapsed());

            match blocks_result {
                Ok(slots) => {
                    if let Some(&latest_slot) = slots.iter().max() {
                        metrics.record_latest_slot(latest_slot);
                    }

                    info!("Found {} new confirmed slots to cache.", slots.len());
                    for slot in slots {
                        cache.insert(slot).await;
                    }
                }
                Err(e) => {
                    warn!("Failed to get confirmed blocks: {}", e);
                }
            }
        }
    });
}

pub fn poll_with_retry<T: RpcApi + 'static + ?Sized>(
    rpc_client: Arc<T>,
    cache: Arc<SlotCache>,
    metrics: Arc<dyn Metrics + Send + Sync>,
    poll_interval: Duration,
    max_retries: u32,
    initial_backoff: Duration,
) {
    info!(
        "Starting background slot poller with retries: {}, and interval: {:?}",
        max_retries, poll_interval
    );

    tokio::spawn(async move {
        loop {
            sleep(poll_interval).await;

            info!("cache: {:?}", cache);

            let latest_on_chain = match rpc_client.get_slot().await {
                Ok(slot) => slot,
                Err(e) => {
                    warn!("Failed to get latest slot from chain: {}", e);
                    continue;
                }
            };
            let start_slot = match cache.get_latest_cached_slot().await {
                Some(latest_cached) => latest_cached + 1,
                None => latest_on_chain.saturating_sub(10),
            };

            if start_slot > latest_on_chain {
                continue;
            }

            let now = Instant::now();
            let blocks_result = with_retry(
                "get_blocks",
                || rpc_client.get_blocks(start_slot, Some(latest_on_chain)),
                max_retries,
                initial_backoff,
            )
            .await;
            metrics.record_get_blocks_elapsed(now.elapsed());

            match blocks_result {
                Ok(slots) => {
                    if let Some(&latest_slot) = slots.iter().max() {
                        metrics.record_latest_slot(latest_slot);
                    }
                    info!("Found {} new confirmed slots to cache.", slots.len());
                    for slot in slots {
                        cache.insert(slot).await;
                    }
                }
                Err(_) => {
                    warn!("get_blocks operation failed after all retries.");
                }
            }
        }
    });
}

pub fn poll_with_transient_retry<T: RpcApi + 'static + ?Sized>(
    rpc_client: Arc<T>,
    cache: Arc<SlotCache>,
    metrics: Arc<dyn Metrics + Send + Sync>,
    poll_interval: Duration,
    max_retries: u32,
    initial_backoff: Duration,
) {
    info!(
        "Starting background slot poller with transient retries: {}, and interval: {:?}",
        max_retries, poll_interval
    );

    tokio::spawn(async move {
        loop {
            sleep(poll_interval).await;

            info!("cache: {:?}", cache);

            let latest_on_chain = match rpc_client.get_slot().await {
                Ok(slot) => slot,
                Err(e) => {
                    warn!("Failed to get latest slot from chain: {}", e);
                    continue;
                }
            };
            let start_slot = match cache.get_latest_cached_slot().await {
                Some(latest_cached) => latest_cached + 1,
                None => latest_on_chain.saturating_sub(10),
            };

            if start_slot > latest_on_chain {
                continue;
            }

            let now = Instant::now();
            let blocks_result = with_transient_retry(
                "get_blocks",
                || rpc_client.get_blocks(start_slot, Some(latest_on_chain)),
                max_retries,
                initial_backoff,
            )
            .await;
            metrics.record_get_blocks_elapsed(now.elapsed());

            match blocks_result {
                Ok(slots) => {
                    if let Some(&latest_slot) = slots.iter().max() {
                        metrics.record_latest_slot(latest_slot);
                    }
                    info!("Found {} new confirmed slots to cache.", slots.len());
                    for slot in slots {
                        cache.insert(slot).await;
                    }
                }
                Err(_) => {
                    warn!("get_blocks operation failed after all transient retries.");
                }
            }
        }
    });
}

pub fn poll_with_transient_retry_and_signals<T: RpcApi + 'static + ?Sized>(
    rpc_client: Arc<T>,
    cache: Arc<SlotCache>,
    metrics: Arc<dyn Metrics + Send + Sync>,
    poll_interval: Duration,
    max_retries: u32,
    initial_backoff: Duration,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    info!(
        "Starting background slot poller with signals, transient retries: {}, and interval: {:?}",
        max_retries, poll_interval
    );

    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received, stopping poller task");
                    break;
                }
                _ = sleep(poll_interval) => {
                }
            }

            info!("cache: {:?}", cache);

            let latest_on_chain = match rpc_client.get_slot().await {
                Ok(slot) => slot,
                Err(e) => {
                    warn!("Failed to get latest slot from chain: {}", e);
                    continue;
                }
            };
            let start_slot = match cache.get_latest_cached_slot().await {
                Some(latest_cached) => latest_cached + 1,
                None => latest_on_chain.saturating_sub(10),
            };

            if start_slot > latest_on_chain {
                continue;
            }

            let now = Instant::now();
            let blocks_result = with_transient_retry(
                "get_blocks",
                || rpc_client.get_blocks(start_slot, Some(latest_on_chain)),
                max_retries,
                initial_backoff,
            )
            .await;
            metrics.record_get_blocks_elapsed(now.elapsed());

            match blocks_result {
                Ok(slots) => {
                    if let Some(&latest_slot) = slots.iter().max() {
                        metrics.record_latest_slot(latest_slot);
                    }
                    info!("Found {} new confirmed slots to cache.", slots.len());
                    for slot in slots {
                        cache.insert(slot).await;
                    }
                }
                Err(_) => {
                    warn!("get_blocks operation failed after all transient retries.");
                }
            }
        }
    });
}

pub fn poll_with_transient_retry_and_signals_and_breaker<T: RpcApi + 'static + ?Sized>(
    rpc_client: Arc<T>,
    cache: Arc<SlotCache>,
    metrics: Arc<dyn Metrics + Send + Sync>,
    circuit_breaker: Arc<CircuitBreaker>,
    poll_interval: Duration,
    max_retries: u32,
    initial_backoff: Duration,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    info!("Starting background slot poller with ALL features enabled.");

    tokio::spawn(async move {
        loop {
            info!("cache: {:?}", cache);

            tokio::select! {
                biased;
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received, stopping final poller task.");
                    break;
                }
                _ = sleep(poll_interval) => {}
            }

            let get_slot_call = || rpc_client.get_slot();
            let latest_on_chain = match circuit_breaker.execute(get_slot_call).await {
                Ok(slot) => slot,
                Err(e) => {
                    warn!("Failed to get latest slot (circuit breaker): {}", e);
                    continue;
                }
            };

            let start_slot = match cache.get_latest_cached_slot().await {
                Some(latest_cached) => latest_cached + 1,
                None => latest_on_chain.saturating_sub(10),
            };

            if start_slot > latest_on_chain {
                continue;
            }

            let now = Instant::now();
            let get_blocks_with_retry_call = || {
                with_transient_retry(
                    "get_blocks",
                    || rpc_client.get_blocks(start_slot, Some(latest_on_chain)),
                    max_retries,
                    initial_backoff,
                )
            };

            let blocks_result = circuit_breaker.execute(get_blocks_with_retry_call).await;
            metrics.record_get_blocks_elapsed(now.elapsed());

            match blocks_result {
                Ok(slots) => {
                    if let Some(&latest_slot) = slots.iter().max() {
                        metrics.record_latest_slot(latest_slot);
                    }
                    info!("Found {} new confirmed slots to cache.", slots.len());
                    for slot in slots {
                        cache.insert(slot).await;
                    }
                }
                Err(e) => {
                    warn!("get_blocks operation failed (circuit breaker): {}", e);
                }
            }
        }
    });
}
