use solana_caching_service::{
    cache::SlotCache, config::Config, rpc::SolanaRpcClient,
    service::slot_poller::start_slot_polling,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env_file(".env").expect("Failed to load config");

    let rpc_url = format!("{}{}", config.rpc_url, config.api_key);
    let real_client = RpcClient::new(rpc_url);
    let rpc_client = Arc::new(SolanaRpcClient::new(real_client));

    let cache = Arc::new(SlotCache::new(config.cache_capacity));

    start_slot_polling(rpc_client.clone(), cache.clone(), config.poll_interval);

    info!("Background service started.");

    std::future::pending::<()>().await;

    Ok(())
}
