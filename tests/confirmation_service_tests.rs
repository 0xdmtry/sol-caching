use mockall::{mock, predicate::*};
use solana_caching_service::{
    cache::{LruCache, SlotCache},
    metrics::Metrics,
    rpc::RpcApi,
    service::confirmation_service::{
        ConfirmationStatus, check_slot_confirmation, check_slot_confirmation_with_lru,
    },
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
async fn test_service_cache_hit() {
    let cache = Arc::new(SlotCache::new(10));
    cache.insert(123).await;
    let lru_cache = Arc::new(LruCache::new(10));
    let mut mock_rpc = MockRpcApi::new();
    mock_rpc.expect_get_blocks().times(0);

    let mut mock_metrics = MockMetrics::new();
    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, cache, lru_cache, mock_metrics);

    let result = check_slot_confirmation(&app_state, 123).await;

    assert_eq!(result, ConfirmationStatus::Confirmed);
}

#[tokio::test]
async fn test_service_cache_miss_rpc_confirmed() {
    let cache = Arc::new(SlotCache::new(10));
    let lru_cache = Arc::new(LruCache::new(10));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc
        .expect_get_blocks()
        .with(eq(456), eq(Some(456)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![456]) }));

    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, cache, lru_cache, mock_metrics);

    let result = check_slot_confirmation(&app_state, 456).await;

    assert_eq!(result, ConfirmationStatus::Confirmed);
}

#[tokio::test]
async fn test_service_cache_miss_rpc_not_confirmed() {
    let cache = Arc::new(SlotCache::new(10));
    let lru_cache = Arc::new(LruCache::new(10));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc
        .expect_get_blocks()
        .with(eq(789), eq(Some(789)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![]) }));

    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, cache, lru_cache, mock_metrics);

    let result = check_slot_confirmation(&app_state, 789).await;

    assert_eq!(result, ConfirmationStatus::NotConfirmed);
}

#[tokio::test]
async fn test_lru_service_hit_in_primary_cache() {
    let primary_cache = Arc::new(SlotCache::new(10));
    primary_cache.insert(100).await;
    let lru_cache = Arc::new(LruCache::new(10));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc.expect_get_blocks().times(0);
    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, primary_cache, lru_cache, mock_metrics);
    let result = check_slot_confirmation_with_lru(&app_state, 100).await;

    assert_eq!(result, ConfirmationStatus::Confirmed);
}

#[tokio::test]
async fn test_lru_service_hit_in_lru_cache() {
    let primary_cache = Arc::new(SlotCache::new(10));
    let lru_cache = Arc::new(LruCache::new(10));
    lru_cache.put(200).await;
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc.expect_get_blocks().times(0);
    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, primary_cache, lru_cache, mock_metrics);
    let result = check_slot_confirmation_with_lru(&app_state, 200).await;

    assert_eq!(result, ConfirmationStatus::Confirmed);
}

#[tokio::test]
async fn test_lru_service_miss_both_caches_rpc_confirms_and_caches() {
    let primary_cache = Arc::new(SlotCache::new(10));
    let lru_cache = Arc::new(LruCache::new(10));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc
        .expect_get_blocks()
        .with(eq(300), eq(Some(300)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![300]) }));
    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, primary_cache, lru_cache.clone(), mock_metrics);

    let result = check_slot_confirmation_with_lru(&app_state, 300).await;

    assert_eq!(result, ConfirmationStatus::Confirmed);
    assert!(
        lru_cache.get(&300).await,
        "Slot should be added to LRU cache on RPC success"
    );
}

#[tokio::test]
async fn test_lru_service_miss_both_caches_rpc_not_confirmed() {
    let primary_cache = Arc::new(SlotCache::new(10));
    let lru_cache = Arc::new(LruCache::new(10));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc
        .expect_get_blocks()
        .with(eq(400), eq(Some(400)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![]) }));
    mock_metrics
        .expect_record_is_slot_confirmed_elapsed()
        .times(1)
        .return_const(());

    let app_state = create_test_app_state(mock_rpc, primary_cache, lru_cache.clone(), mock_metrics);

    let result = check_slot_confirmation_with_lru(&app_state, 400).await;

    assert_eq!(result, ConfirmationStatus::NotConfirmed);
    assert!(
        !lru_cache.get(&400).await,
        "Slot should not be added to LRU cache on RPC failure"
    );
}
