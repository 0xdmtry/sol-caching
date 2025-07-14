use solana_caching_service::utils::{
    error_utils::IsTransient,
    retry::{with_retry, with_transient_retry},
};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, PartialEq, Clone)]
struct TestError {
    message: String,
    is_transient: bool,
}

impl IsTransient for TestError {
    fn is_transient(&self) -> bool {
        self.is_transient
    }
}

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

#[tokio::test]
async fn test_transient_retry_fails_immediately_on_non_transient_error() {
    let attempts = Arc::new(Mutex::new(0));
    let operation = || {
        let attempts_clone = attempts.clone();
        async move {
            let _ = {
                *attempts_clone.lock().unwrap() += 1;
            };
            Err(TestError {
                message: "Permanent failure".into(),
                is_transient: false,
            })
        }
    };

    let result: Result<(), _> =
        with_transient_retry("test_non_transient", operation, 5, Duration::from_millis(1)).await;

    assert!(result.is_err());
    assert_eq!(
        *attempts.lock().unwrap(),
        1,
        "Operation should only be called once"
    );
}

#[tokio::test]
async fn test_transient_retry_succeeds_after_transient_error() {
    let attempts = Arc::new(Mutex::new(0));
    let operation = || {
        let attempts_clone = attempts.clone();
        async move {
            let current_attempt = {
                let mut num = attempts_clone.lock().unwrap();
                *num += 1;
                *num
            };
            if current_attempt < 2 {
                Err(TestError {
                    message: "Temporary failure".into(),
                    is_transient: true,
                })
            } else {
                Ok("Success")
            }
        }
    };

    let result =
        with_transient_retry("test_transient", operation, 5, Duration::from_millis(1)).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
    assert_eq!(
        *attempts.lock().unwrap(),
        2,
        "Operation should be called twice"
    );
}
