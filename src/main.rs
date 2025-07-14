use solana_caching_service::metrics::LoggingMetrics;
use solana_caching_service::metrics::Metrics;
use solana_caching_service::{
    cache::SlotCache, config::Config, routes::create_router, rpc::RpcApi,
    service::slot_poller::start_slot_polling, service::slot_poller::start_slot_polling_with_retry,
    state::AppState,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env_file(".env").expect("Failed to load config");

    let rpc_url = format!("{}{}", config.rpc_url, config.api_key);
    let rpc_client: Arc<dyn RpcApi + Send + Sync> = Arc::new(RpcClient::new(rpc_url));
    let cache = Arc::new(SlotCache::new(config.cache_capacity));
    let metrics: Arc<dyn Metrics + Send + Sync> = Arc::new(LoggingMetrics);

    start_slot_polling_with_retry(
        rpc_client.clone(),
        cache.clone(),
        metrics.clone(),
        config.poll_interval,
        config.max_retries,
        config.initial_backoff,
    );

    let app_state = AppState {
        rpc_client,
        cache,
        metrics,
    };

    let app = create_router(app_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = TcpListener::bind(addr).await?;

    info!("Running server on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
