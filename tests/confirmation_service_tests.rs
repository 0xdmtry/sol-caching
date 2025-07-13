use mockall::{mock, predicate::*};
use solana_caching_service::{
    cache::SlotCache,
    rpc::RpcApi,
    service::confirmation_service::{ConfirmationStatus, check_slot_confirmation},
    state::AppState,
};
use solana_client::client_error::ClientError;
use std::{future::Future, pin::Pin, sync::Arc};

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

fn create_test_app_state(mock_rpc: MockRpcApi, cache: Arc<SlotCache>) -> AppState {
    AppState {
        rpc_client: Arc::new(mock_rpc),
        cache,
    }
}

#[tokio::test]
async fn test_service_cache_hit() {
    let cache = Arc::new(SlotCache::new(10));
    cache.insert(123).await;

    let mut mock_rpc = MockRpcApi::new();
    mock_rpc.expect_get_blocks().times(0);

    let app_state = create_test_app_state(mock_rpc, cache);

    let result = check_slot_confirmation(&app_state, 123).await;

    assert_eq!(result, ConfirmationStatus::Confirmed);
}

#[tokio::test]
async fn test_service_cache_miss_rpc_confirmed() {
    let cache = Arc::new(SlotCache::new(10));
    let mut mock_rpc = MockRpcApi::new();

    mock_rpc
        .expect_get_blocks()
        .with(eq(456), eq(Some(456)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![456]) }));

    let app_state = create_test_app_state(mock_rpc, cache);

    let result = check_slot_confirmation(&app_state, 456).await;

    assert_eq!(result, ConfirmationStatus::Confirmed);
}

#[tokio::test]
async fn test_service_cache_miss_rpc_not_confirmed() {
    let cache = Arc::new(SlotCache::new(10));
    let mut mock_rpc = MockRpcApi::new();

    mock_rpc
        .expect_get_blocks()
        .with(eq(789), eq(Some(789)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![]) }));

    let app_state = create_test_app_state(mock_rpc, cache);

    let result = check_slot_confirmation(&app_state, 789).await;

    assert_eq!(result, ConfirmationStatus::NotConfirmed);
}
