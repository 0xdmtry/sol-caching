use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub api_key: String,
    pub poll_interval: Duration,
    pub cache_capacity: usize,
}

impl Config {
    pub fn from_env_file(path: &str) -> Result<Self, Error> {
        let vars = load_dotenv(path)?;

        let rpc_url = vars
            .get("SOLANA_RPC_URL")
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "SOLANA_RPC_URL is not set"))?
            .clone();

        let api_key = vars
            .get("API_KEY")
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "API_KEY is not set"))?
            .clone();

        let poll_interval_seconds = vars
            .get("POLL_INTERVAL_SECONDS")
            .and_then(|s| s.parse().ok())
            .unwrap_or(15);
        let poll_interval = Duration::from_secs(poll_interval_seconds);

        let cache_capacity = vars
            .get("CACHE_CAPACITY")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        Ok(Config {
            rpc_url,
            api_key,
            poll_interval,
            cache_capacity,
        })
    }
}

fn load_dotenv(path: &str) -> Result<HashMap<String, String>, Error> {
    let mut vars: HashMap<String, String> = HashMap::new();
    let content = fs::read_to_string(path)?;

    for line in content.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            vars.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    Ok(vars)
}
