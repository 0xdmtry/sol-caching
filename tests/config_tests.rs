use solana_caching_service::config::Config;
use std::fs;
use std::io::{ErrorKind, Write};

fn create_temp_env_file(content: &str, file_path: &str) {
    let mut file = fs::File::create(&file_path).expect("Failed to create temp env file");
    file.write_all(content.as_bytes())
        .expect("Failed to write to temp env file");
}

#[test]
fn test_from_env_file_success() {
    let file_path = "test_success.env";
    let content = "SOLANA_RPC_URL=http://example.com/\nAPI_KEY=12345";

    create_temp_env_file(content, file_path);

    let config = Config::from_env_file(file_path).unwrap();

    assert_eq!(config.rpc_url, "http://example.com/");
    assert_eq!(config.api_key, "12345");

    fs::remove_file(file_path).unwrap();
}

#[test]
fn test_from_env_file_missing_key() {
    let file_path = "test_missing_key.env";
    let content = "SOLANA_RPC_URL=http://example.com/";

    create_temp_env_file(content, file_path);

    let result = Config::from_env_file(file_path);

    assert!(result.is_err());
    assert_eq!(result.err().unwrap().kind(), ErrorKind::NotFound);

    fs::remove_file(file_path).unwrap();
}

#[test]
fn test_config_fails_if_retry_duration_exceeds_interval() {
    let file_path = "test_invalid_retry.env";
    let content = "
SOLANA_RPC_URL=http://test.com/
API_KEY=12345
POLL_INTERVAL_SECONDS=7
MAX_RETRIES=3
INITIAL_BACKOFF_MS=1000
CACHE_CAPACITY=100
    ";
    create_temp_env_file(content, file_path);

    let result = Config::from_env_file(file_path);

    assert!(result.is_err());
    assert_eq!(result.err().unwrap().kind(), ErrorKind::InvalidInput);

    fs::remove_file(file_path).unwrap();
}

#[test]
fn test_config_succeeds_if_retry_duration_is_valid() {
    let file_path = "test_valid_retry.env";
    let content = "
SOLANA_RPC_URL=http://test.com/
API_KEY=12345
POLL_INTERVAL_SECONDS=8
MAX_RETRIES=3
INITIAL_BACKOFF_MS=1000
CACHE_CAPACITY=100
    ";
    create_temp_env_file(content, file_path);

    let result = Config::from_env_file(file_path);

    assert!(result.is_ok());

    fs::remove_file(file_path).unwrap();
}
