use solana_caching_service::{
    cache::SlotCache, config::Config, routes::create_router, rpc::SolanaRpcClient,
    service::slot_poller::start_slot_polling, state::AppState,
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
    let rpc_client = Arc::new(SolanaRpcClient::new(RpcClient::new(rpc_url)));
    let cache = Arc::new(SlotCache::new(config.cache_capacity));

    start_slot_polling(rpc_client.clone(), cache.clone(), config.poll_interval);

    let app_state = AppState { rpc_client, cache };

    let app = create_router(app_state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let listener = TcpListener::bind(addr).await?;

    info!("Running server on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
