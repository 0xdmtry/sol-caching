use solana_caching_service::circuit_breaker::CircuitBreaker;
use solana_caching_service::metrics::LoggingMetrics;
use solana_caching_service::metrics::Metrics;
use solana_caching_service::{
    cache::{LruCache, SlotCache},
    config::Config,
    routes::create_router,
    rpc::RpcApi,
    service::slot_poller::poll_with_transient_retry_and_signals_and_breaker,
    signals::shutdown_signal,
    state::AppState,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env_file(".env").expect("Failed to load config");

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

    let rpc_url = format!("{}{}", config.rpc_url, config.api_key);
    let rpc_client: Arc<dyn RpcApi + Send + Sync> = Arc::new(RpcClient::new(rpc_url));
    let cache = Arc::new(SlotCache::new(config.cache_capacity));
    let lru_cache = Arc::new(LruCache::new(config.lru_cache_capacity));
    let metrics: Arc<dyn Metrics + Send + Sync> = Arc::new(LoggingMetrics);
    let circuit_breaker = Arc::new(CircuitBreaker::new(
        config.circuit_failure_threshold,
        config.circuit_open_duration,
    ));

    poll_with_transient_retry_and_signals_and_breaker(
        rpc_client.clone(),
        cache.clone(),
        metrics.clone(),
        circuit_breaker.clone(),
        config.poll_interval,
        config.max_retries,
        config.initial_backoff,
        shutdown_rx,
    );

    let app_state = AppState {
        rpc_client,
        cache,
        lru_cache,
        metrics,
        circuit_breaker,
    };

    let app = create_router(app_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = TcpListener::bind(addr).await?;

    info!("Running server on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            let _ = shutdown_tx.send(());
        })
        .await?;

    Ok(())
}
