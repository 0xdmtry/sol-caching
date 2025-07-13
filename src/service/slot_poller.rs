use crate::{cache::SlotCache, rpc::RpcApi};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{info, warn};

pub fn start_slot_polling<T: RpcApi + 'static + ?Sized>(
    rpc_client: Arc<T>,
    cache: Arc<SlotCache>,
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

            match rpc_client
                .get_blocks(start_slot, Some(latest_on_chain))
                .await
            {
                Ok(slots) => {
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
