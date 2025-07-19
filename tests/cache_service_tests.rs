use mockall::mock;
use solana_caching_service::{
    cache::{LruCache, SlotCache},
    circuit_breaker::CircuitBreaker,
    metrics::Metrics,
    rpc::RpcApi,
    service::cache_service::{get_all_latest_slots, get_all_lru_slots},
    state::AppState,
};
use solana_client::client_error::ClientError;
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

mock! {
    pub RpcApi {}
    impl RpcApi for RpcApi {
        fn get_slot<'a>(&'a self) -> Pin<Box<dyn Future<Output = Result<u64, ClientError>> + Send + 'a>>;
        fn get_blocks<'a>(
            &'a self,
            start_slot: u64,
            end_slot: Option<u64>,
        ) -> Pin<Box<dyn Future<Output = Result<Vec<u64>, ClientError>> + Send + 'a>>;
    }
}
mock! {
    pub Metrics {}
    impl Metrics for Metrics {
        fn record_latest_slot(&self, slot: u64);
        fn record_get_blocks_elapsed(&self, elapsed: Duration);
        fn record_is_slot_confirmed_elapsed(&self, elapsed: Duration);
    }
}

#[tokio::test]
async fn test_get_all_latest_slots_service() {
    let slot_cache = Arc::new(SlotCache::new(10));
    slot_cache.insert(100).await;
    slot_cache.insert(101).await;

    let app_state = AppState {
        cache: slot_cache,
        lru_cache: Arc::new(LruCache::new(10)),
        rpc_client: Arc::new(MockRpcApi::new()),
        metrics: Arc::new(MockMetrics::new()),
        circuit_breaker: Arc::new(CircuitBreaker::new(3, Duration::from_secs(10))),
    };

    let result = get_all_latest_slots(&app_state).await;

    assert_eq!(result, vec![100, 101]);
}

#[tokio::test]
async fn test_get_all_lru_slots_service() {
    let lru_cache = Arc::new(LruCache::new(10));
    lru_cache.put(200).await;
    lru_cache.put(201).await;

    let app_state = AppState {
        lru_cache,
        cache: Arc::new(SlotCache::new(10)),
        rpc_client: Arc::new(MockRpcApi::new()),
        metrics: Arc::new(MockMetrics::new()),
        circuit_breaker: Arc::new(CircuitBreaker::new(3, Duration::from_secs(10))),
    };

    let result = get_all_lru_slots(&app_state).await;

    assert_eq!(result, vec![201, 200]);
}
