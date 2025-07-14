use solana_caching_service::utils::retry::with_retry;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::test]
async fn test_retry_succeeds_on_third_attempt() {
    let attempts = Arc::new(Mutex::new(0));

    let operation = || {
        let attempts_clone = attempts.clone();
        async move {
            let current_attempt = {
                let mut num = attempts_clone.lock().unwrap();
                *num += 1;
                *num
            };

            if current_attempt < 3 {
                Err("Operation failed")
            } else {
                Ok("Success")
            }
        }
    };

    let result = with_retry("test_op", operation, 5, Duration::from_millis(1)).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(
        *attempts.lock().unwrap(),
        3,
        "Operation should have been called 3 times"
    );
}

#[tokio::test]
async fn test_retry_fails_after_all_attempts() {
    let attempts = Arc::new(Mutex::new(0));

    let operation = || {
        let attempts_clone = attempts.clone();
        async move {
            let _ = {
                let mut num = attempts_clone.lock().unwrap();
                *num += 1;
            };
            Err("Always fails")
        }
    };

    let result: Result<(), _> =
        with_retry("test_op_fail", operation, 2, Duration::from_millis(1)).await;

    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), "Always fails");

    assert_eq!(
        *attempts.lock().unwrap(),
        3,
        "Operation should have been called 3 times"
    );
}
