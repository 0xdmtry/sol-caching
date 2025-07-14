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
    pub max_retries: u32,
    pub initial_backoff: Duration,
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

        let max_retries = vars
            .get("MAX_RETRIES")
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);

        let initial_backoff_ms = vars
            .get("INITIAL_BACKOFF_MS")
            .and_then(|s| s.parse().ok())
            .unwrap_or(500);
        let initial_backoff = Duration::from_millis(initial_backoff_ms);

        // Calculate max possible retries
        let max_retry_sleep_duration_ms: u64 = (0..max_retries)
            .map(|i| initial_backoff_ms * 2_u64.pow(i))
            .sum();

        // Compare max possible retries with requested max retries
        // If the requested is smaller of max possible, error is returned
        if max_retry_sleep_duration_ms >= poll_interval.as_millis() as u64 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Config error: Max possible retries exceeds the polling interval: either increase POLL_INTERVAL_SECONDS or decrease MAX_RETRIES/ INITIAL_BACKOFF_MS",
            ));
        }

        Ok(Config {
            rpc_url,
            api_key,
            poll_interval,
            cache_capacity,
            max_retries,
            initial_backoff,
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
