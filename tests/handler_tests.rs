use axum::response::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use mockall::{mock, predicate::*};
use solana_caching_service::{
    cache::{LruCache, SlotCache},
    handler::slot_handler::check_slot_confirmation_handler,
    metrics::Metrics,
    rpc::RpcApi,
    state::AppState,
};
use solana_client::client_error::{ClientError, ClientErrorKind};
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

fn create_test_app_state(
    mock_rpc: MockRpcApi,
    cache: Arc<SlotCache>,
    lru_cache: Arc<LruCache>,
    mock_metrics: MockMetrics,
) -> AppState {
    AppState {
        rpc_client: Arc::new(mock_rpc),
        cache,
        lru_cache,
        metrics: Arc::new(mock_metrics),
    }
}

#[tokio::test]
async fn test_handler_returns_200_ok_on_confirmed() {
    let cache = Arc::new(SlotCache::new(10));
    cache.insert(100).await;
    let lru_cache = Arc::new(LruCache::new(10));
    let mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, cache, lru_cache, mock_metrics);

    let response = check_slot_confirmation_handler(State(app_state), Path(100))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_handler_returns_404_not_found() {
    let cache = Arc::new(SlotCache::new(10));
    let lru_cache = Arc::new(LruCache::new(10));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc
        .expect_get_blocks()
        .returning(|_, _| Box::pin(async { Ok(vec![]) }));

    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, cache, lru_cache, mock_metrics);

    let response = check_slot_confirmation_handler(State(app_state), Path(200))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_handler_returns_500_on_rpc_error() {
    let cache = Arc::new(SlotCache::new(10));
    let lru_cache = Arc::new(LruCache::new(10));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc.expect_get_blocks().returning(|_, _| {
        let rpc_error = ClientError {
            kind: ClientErrorKind::Custom("RPC down".to_string()),
            request: None,
        };
        Box::pin(async { Err(rpc_error) })
    });

    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, cache, lru_cache, mock_metrics);

    let response = check_slot_confirmation_handler(State(app_state), Path(300))
        .await
        .into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
