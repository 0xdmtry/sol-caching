use mockall::{Sequence, mock, predicate::*};
use solana_caching_service::{
    cache::SlotCache,
    circuit_breaker::CircuitBreaker,
    metrics::Metrics,
    rpc::RpcApi,
    service::slot_poller::{
        poll, poll_with_retry, poll_with_transient_retry, poll_with_transient_retry_and_signals,
        poll_with_transient_retry_and_signals_and_breaker,
    },
};
use solana_client::client_error::{ClientError, ClientErrorKind};
use std::{future::Future, io, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::broadcast;

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
async fn test_poller_populates_empty_cache() {
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();
    let cache = Arc::new(SlotCache::new(20));

    mock_rpc
        .expect_get_slot()
        .times(1)
        .returning(|| Box::pin(async { Ok(100) }));

    mock_rpc
        .expect_get_blocks()
        .with(eq(90), eq(Some(100)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![95, 96, 98]) }));

    mock_metrics
        .expect_record_get_blocks_elapsed()
        .times(1)
        .return_const(());
    mock_metrics
        .expect_record_latest_slot()
        .with(eq(98))
        .times(1)
        .return_const(());

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll(
        rpc_client,
        cache.clone(),
        metrics,
        Duration::from_millis(10),
    );
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(cache.contains(&95).await);
    assert!(cache.contains(&96).await);
    assert!(cache.contains(&98).await);
    assert!(!cache.contains(&97).await);
}

#[tokio::test]
async fn test_poller_fetches_from_latest_cached() {
    let cache = Arc::new(SlotCache::new(20));
    cache.insert(100).await;

    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc
        .expect_get_slot()
        .times(1)
        .returning(|| Box::pin(async { Ok(105) }));

    mock_rpc
        .expect_get_blocks()
        .with(eq(101), eq(Some(105)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![102, 104]) }));

    mock_metrics
        .expect_record_get_blocks_elapsed()
        .times(1)
        .return_const(());
    mock_metrics
        .expect_record_latest_slot()
        .with(eq(104))
        .times(1)
        .return_const(());

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll(
        rpc_client,
        cache.clone(),
        metrics,
        Duration::from_millis(10),
    );
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(cache.contains(&100).await);
    assert!(cache.contains(&102).await);
    assert!(cache.contains(&104).await);
}

#[tokio::test]
async fn test_poller_does_nothing_if_up_to_date() {
    let cache = Arc::new(SlotCache::new(20));
    cache.insert(100).await;

    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc
        .expect_get_slot()
        .times(1)
        .returning(|| Box::pin(async { Ok(100) }));

    mock_rpc.expect_get_blocks().times(0);

    mock_metrics.expect_record_get_blocks_elapsed().times(0);
    mock_metrics.expect_record_latest_slot().times(0);

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll(
        rpc_client,
        cache.clone(),
        metrics,
        Duration::from_millis(10),
    );
    tokio::time::sleep(Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_poller_handles_rpc_error() {
    let cache = Arc::new(SlotCache::new(20));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();
    let mut seq = Sequence::new();

    mock_rpc
        .expect_get_slot()
        .times(1)
        .in_sequence(&mut seq)
        .returning(move || {
            let rpc_error = ClientError {
                kind: ClientErrorKind::Custom("RPC down".to_string()),
                request: None,
            };
            Box::pin(async { Err(rpc_error) })
        });

    mock_rpc
        .expect_get_slot()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|| Box::pin(async { Ok(50) }));

    mock_rpc
        .expect_get_blocks()
        .with(eq(40), eq(Some(50)))
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![45]) }));

    mock_metrics
        .expect_record_get_blocks_elapsed()
        .times(1)
        .return_const(());
    mock_metrics
        .expect_record_latest_slot()
        .with(eq(45))
        .times(1)
        .return_const(());

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll(
        rpc_client,
        cache.clone(),
        metrics,
        Duration::from_millis(20),
    );
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(cache.contains(&45).await);
}

#[tokio::test]
async fn test_poller_with_retry_succeeds_after_failures() {
    let cache = Arc::new(SlotCache::new(20));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();
    let mut seq = Sequence::new();

    mock_rpc
        .expect_get_blocks()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| {
            let err = ClientError {
                kind: ClientErrorKind::Custom("fail 1".into()),
                request: None,
            };
            Box::pin(async { Err(err) })
        });

    mock_rpc
        .expect_get_blocks()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| {
            let err = ClientError {
                kind: ClientErrorKind::Custom("fail 2".into()),
                request: None,
            };
            Box::pin(async { Err(err) })
        });

    mock_rpc
        .expect_get_blocks()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| Box::pin(async { Ok(vec![205, 208]) }));

    mock_rpc
        .expect_get_slot()
        .times(1)
        .returning(|| Box::pin(async { Ok(210) }));

    mock_metrics
        .expect_record_get_blocks_elapsed()
        .times(1)
        .return_const(());
    mock_metrics
        .expect_record_latest_slot()
        .with(eq(208))
        .times(1)
        .return_const(());

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll_with_retry(
        rpc_client,
        cache.clone(),
        metrics,
        Duration::from_millis(10),
        3,
        Duration::from_millis(5),
    );

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(cache.contains(&205).await);
    assert!(cache.contains(&208).await);
}

#[tokio::test]
async fn test_transient_poller_retries_on_transient_error() {
    let cache = Arc::new(SlotCache::new(20));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();
    let mut seq = Sequence::new();

    mock_rpc
        .expect_get_blocks()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| {
            // Create a standard library I/O error to simulate a transient failure.
            let io_error = io::Error::new(io::ErrorKind::TimedOut, "test timeout");
            let err = ClientError {
                kind: ClientErrorKind::Io(io_error),
                request: None,
            };
            Box::pin(async { Err(err) })
        });

    mock_rpc
        .expect_get_blocks()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| Box::pin(async { Ok(vec![205, 208]) }));

    mock_rpc
        .expect_get_slot()
        .times(1)
        .returning(|| Box::pin(async { Ok(210) }));
    mock_metrics
        .expect_record_get_blocks_elapsed()
        .times(1)
        .return_const(());
    mock_metrics
        .expect_record_latest_slot()
        .with(eq(208))
        .times(1)
        .return_const(());

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll_with_transient_retry(
        rpc_client,
        cache.clone(),
        metrics,
        Duration::from_millis(10),
        3,
        Duration::from_millis(5),
    );

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(cache.contains(&205).await);
    assert!(cache.contains(&208).await);
}

#[tokio::test]
async fn test_transient_poller_fails_immediately_on_non_transient_error() {
    let cache = Arc::new(SlotCache::new(20));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    mock_rpc.expect_get_blocks().times(1).returning(|_, _| {
        let err = ClientError {
            kind: ClientErrorKind::Custom("Permanent error".into()),
            request: None,
        };
        Box::pin(async { Err(err) })
    });

    mock_rpc
        .expect_get_slot()
        .times(1)
        .returning(|| Box::pin(async { Ok(210) }));
    mock_metrics
        .expect_record_get_blocks_elapsed()
        .times(1)
        .return_const(());
    mock_metrics.expect_record_latest_slot().times(0);

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll_with_transient_retry(
        rpc_client,
        cache.clone(),
        metrics,
        Duration::from_millis(10),
        3,
        Duration::from_millis(5),
    );

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(cache.get_latest_cached_slot().await, None);
}

#[tokio::test]
async fn test_poller_with_signals_shuts_down_gracefully() {
    let cache = Arc::new(SlotCache::new(20));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    mock_rpc
        .expect_get_slot()
        .times(1)
        .returning(|| Box::pin(async { Ok(100) }));
    mock_rpc
        .expect_get_blocks()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![95]) }));
    mock_metrics
        .expect_record_get_blocks_elapsed()
        .times(1)
        .return_const(());
    mock_metrics
        .expect_record_latest_slot()
        .times(1)
        .return_const(());

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll_with_transient_retry_and_signals(
        rpc_client,
        cache.clone(),
        metrics,
        Duration::from_millis(20),
        3,
        Duration::from_millis(5),
        shutdown_rx,
    );

    tokio::time::sleep(Duration::from_millis(30)).await;

    assert!(cache.contains(&95).await);

    let _ = shutdown_tx.send(());

    tokio::time::sleep(Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_final_poller_works_when_circuit_is_closed() {
    // Arrange
    let cache = Arc::new(SlotCache::new(20));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, Duration::from_secs(10)));
    let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);

    mock_rpc
        .expect_get_slot()
        .times(1)
        .returning(|| Box::pin(async { Ok(100) }));
    mock_rpc
        .expect_get_blocks()
        .times(1)
        .returning(|_, _| Box::pin(async { Ok(vec![95]) }));
    mock_metrics
        .expect_record_get_blocks_elapsed()
        .times(1)
        .return_const(());
    mock_metrics
        .expect_record_latest_slot()
        .times(1)
        .return_const(());

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll_with_transient_retry_and_signals_and_breaker(
        rpc_client,
        cache.clone(),
        metrics,
        circuit_breaker,
        Duration::from_millis(10),
        3,
        Duration::from_millis(5),
        shutdown_rx,
    );

    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(cache.contains(&95).await);
}

#[tokio::test]
async fn test_final_poller_is_blocked_by_open_circuit() {
    let cache = Arc::new(SlotCache::new(20));
    let mut mock_rpc = MockRpcApi::new();
    let mut mock_metrics = MockMetrics::new();
    let circuit_breaker = Arc::new(CircuitBreaker::new(1, Duration::from_secs(10)));
    let (_shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let mut seq = Sequence::new();

    mock_rpc
        .expect_get_slot()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|| Box::pin(async { Ok(100) }));
    mock_rpc
        .expect_get_blocks()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_, _| {
            let err = ClientError {
                kind: ClientErrorKind::Custom("RPC Down".into()),
                request: None,
            };
            Box::pin(async { Err(err) })
        });

    mock_metrics
        .expect_record_get_blocks_elapsed()
        .times(1)
        .return_const(());
    mock_metrics.expect_record_latest_slot().times(0);

    let rpc_client = Arc::new(mock_rpc);
    let metrics = Arc::new(mock_metrics);

    poll_with_transient_retry_and_signals_and_breaker(
        rpc_client,
        cache.clone(),
        metrics,
        circuit_breaker,
        Duration::from_millis(20),
        0,
        Duration::from_millis(5),
        shutdown_rx,
    );

    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(cache.get_latest_cached_slot().await, None);
}
