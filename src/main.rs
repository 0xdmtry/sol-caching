use solana_caching_service::config::Config;
use solana_caching_service::rpc::SolanaRpcClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use std::process;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = match Config::from_env_file(".env") {
        Ok(cfg) => {
            info!("Config is set successfully");
            cfg
        }
        Err(e) => {
            error!("Failed to load config from .env: {}", e);
            process::exit(1);
        }
    };

    let rpc_url = format!("{}{}", config.rpc_url, config.api_key);
    let rpc_client = RpcClient::new(rpc_url);
    let client = SolanaRpcClient::new(rpc_client);

    info!("Pinning RPC...");

    // Test 1: get_latest_slot
    let latest_slot = match client.get_latest_slot().await {
        Ok(slot) => {
            info!("Slot: {}", slot);
            slot
        }
        Err(e) => {
            error!("Failed to get slot: {}", e);
            process::exit(1);
        }
    };

    // Test 2: get_confirmed_blocks
    let start_slot = latest_slot.saturating_sub(10);

    info!(
        "Fetching confirmed blocks from slot {} to {}",
        start_slot, latest_slot
    );

    match client
        .get_confirmed_blocks(start_slot, Some(latest_slot))
        .await
    {
        Ok(blocks) => {
            info!("Confirmed blocks: {:?}", blocks);
        }
        Err(e) => {
            error!("Failed to get confirmed blocks: {}", e)
        }
    }

    Ok(())
}
