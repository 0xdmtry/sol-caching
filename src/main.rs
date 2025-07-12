use solana_caching_service::config::Config;
use std::process;
use tracing::{error, info};
use solana_client::nonblocking::rpc_client::RpcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
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
    let client = RpcClient::new(rpc_url);

    info!("Pinning RPC...");

    match client.get_slot().await {
        Ok(slot) => {
            info!("Slot: {}", slot);
        }
        Err(e) => {
            error!("Failed to get slot: {}", e);
        }
    }

    Ok(())
}
