use axum::{extract::State, http::StatusCode, response::IntoResponse};
use mockall::mock;
use solana_caching_service::{
    cache::{LruCache, SlotCache},
    circuit_breaker::CircuitBreaker,
    handler::cache_handler::{get_latest_slots_handler, get_lru_slots_handler},
    metrics::Metrics,
    rpc::RpcApi,
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

fn create_test_app_state() -> AppState {
    AppState {
        cache: Arc::new(SlotCache::new(10)),
        lru_cache: Arc::new(LruCache::new(10)),
        rpc_client: Arc::new(MockRpcApi::new()),
        metrics: Arc::new(MockMetrics::new()),
        circuit_breaker: Arc::new(CircuitBreaker::new(3, Duration::from_secs(10))),
    }
}

#[tokio::test]
async fn test_get_latest_slots_handler() {
    let app_state = create_test_app_state();
    app_state.cache.insert(100).await;
    app_state.cache.insert(101).await;

    let response = get_latest_slots_handler(State(app_state))
        .await
        .into_response();
    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body_str, "[100,101]");
}

#[tokio::test]
async fn test_get_lru_slots_handler() {
    let app_state = create_test_app_state();
    app_state.lru_cache.put(200).await;
    app_state.lru_cache.put(201).await;

    let response = get_lru_slots_handler(State(app_state))
        .await
        .into_response();
    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body_str, "[201,200]");
}
