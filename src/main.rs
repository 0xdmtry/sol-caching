use solana_caching_service::config::Config;
use std::process;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    tracing_subscriber::fmt::init();

    let config = match Config::from_env_file(".env") {
        Ok(cfg) => {
            cfg
        }
        Err(e) => {
            error!("Failed to load config from .env: {}", e);
            process::exit(1);
        }
    };

    println!("{:?}", config);

    Ok(())
}
