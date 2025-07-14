use crate::utils::error_utils::IsTransient;
use std::{future::Future, time::Duration};
use tokio::time::sleep;
use tracing::{error, warn};

pub async fn with_retry<F, Fut, T, E>(
    operation_name: &str,
    operation: F,
    max_retries: u32,
    initial_backoff: Duration,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut attempts = 0;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                attempts += 1;
                if attempts > max_retries {
                    return Err(e);
                }

                let backoff_duration = initial_backoff * 2_u32.pow(attempts - 1);
                warn!(
                    "Operation '{}' failed (attempt {}/{}). Retrying in {:?}...",
                    operation_name, attempts, max_retries, backoff_duration
                );
                sleep(backoff_duration).await;
            }
        }
    }
}

pub async fn with_transient_retry<F, Fut, T, E>(
    operation_name: &str,
    operation: F,
    max_retries: u32,
    initial_backoff: Duration,
) -> Result<T, E>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: IsTransient,
{
    let mut attempts = 0;
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                if !e.is_transient() {
                    error!(
                        "Operation '{}' failed with a non-transient error: {:?}. Will not retry.",
                        operation_name, e
                    );
                    return Err(e);
                }

                attempts += 1;
                if attempts > max_retries {
                    error!(
                        "Operation '{}' failed after all {} retries. Last error: {:?}",
                        operation_name, max_retries, e
                    );
                    return Err(e);
                }

                let backoff_duration = initial_backoff * 2_u32.pow(attempts - 1);
                warn!(
                    "Operation '{}' failed with transient error (attempt {}/{}): {:?}. Retrying in {:?}...",
                    operation_name, attempts, max_retries, e, backoff_duration
                );
                sleep(backoff_duration).await;
            }
        }
    }
}
