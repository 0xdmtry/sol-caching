use mockall::{mock, predicate::*};
use solana_caching_service::rpc::{RpcApi, SolanaRpcClient};
use solana_client::client_error::ClientError;
use std::{future::Future, pin::Pin};

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

#[tokio::test]
async fn test_get_latest_slot_success() {
    let mut mock = MockRpcApi::new();
    mock.expect_get_slot()
        .times(1)
        .returning(|| Box::pin(async { Ok(12345) }));
    let client = SolanaRpcClient::new(mock);
    assert_eq!(client.get_latest_slot().await.unwrap(), 12345);
}

#[tokio::test]
async fn test_get_confirmed_blocks_success() {
    let mut mock = MockRpcApi::new();
    mock.expect_get_blocks()
        .with(eq(90), eq(Some(100)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![92, 95, 98]) }));
    let client = SolanaRpcClient::new(mock);
    let result = client.get_confirmed_blocks(90, Some(100)).await;
    assert_eq!(result.unwrap(), vec![92, 95, 98]);
}
