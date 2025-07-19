use solana_caching_service::circuit_breaker::{CircuitBreaker, CircuitBreakerError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
struct TestError(String);

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for TestError {}

#[tokio::test]
async fn test_circuit_stays_closed_on_success() {
    let circuit_breaker = CircuitBreaker::new(3, Duration::from_secs(10));
    let operation = || async { Ok::<_, TestError>("Success") };

    let result = circuit_breaker.execute(operation).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
}

#[tokio::test]
async fn test_circuit_opens_after_threshold_failures() {
    let failure_threshold = 2;
    let circuit_breaker = CircuitBreaker::new(failure_threshold, Duration::from_secs(10));
    let failing_operation = || async { Err(TestError("Failed".into())) };

    for _ in 0..failure_threshold {
        let result: Result<(), _> = circuit_breaker.execute(failing_operation).await;
        assert!(matches!(result, Err(CircuitBreakerError::Inner(_))));
    }

    let result_after_opening: Result<(), _> = circuit_breaker.execute(failing_operation).await;

    assert!(matches!(
        result_after_opening,
        Err(CircuitBreakerError::CircuitOpen)
    ));
}

#[tokio::test]
async fn test_circuit_transitions_to_half_open_and_then_closes() {
    let open_duration = Duration::from_millis(20);
    let circuit_breaker = CircuitBreaker::new(1, open_duration);

    let attempts = Arc::new(Mutex::new(0));
    let operation = || {
        let attempts_clone = attempts.clone();
        async move {
            let mut num = attempts_clone.lock().unwrap();
            *num += 1;
            if *num <= 1 {
                Err(TestError("Initial failure".into()))
            } else {
                Ok("Probe success")
            }
        }
    };

    let _ = circuit_breaker.execute(operation).await;
    let result_while_open: Result<&str, _> = circuit_breaker.execute(operation).await;
    assert!(matches!(
        result_while_open,
        Err(CircuitBreakerError::CircuitOpen)
    ));

    tokio::time::sleep(open_duration + Duration::from_millis(5)).await;

    let probe_result = circuit_breaker.execute(operation).await;

    assert!(probe_result.is_ok());
    assert_eq!(probe_result.unwrap(), "Probe success");

    let final_result: Result<&str, CircuitBreakerError<TestError>> = circuit_breaker
        .execute(|| async { Ok("Final success") })
        .await;

    assert!(final_result.is_ok());
}

#[tokio::test]
async fn test_circuit_transitions_to_half_open_and_back_to_open() {
    let open_duration = Duration::from_millis(20);
    let circuit_breaker = CircuitBreaker::new(1, open_duration);
    let failing_operation = || async { Err(TestError("Always fails".into())) };

    let _: Result<(), _> = circuit_breaker.execute(failing_operation).await;

    tokio::time::sleep(open_duration + Duration::from_millis(5)).await;

    let probe_result: Result<(), _> = circuit_breaker.execute(failing_operation).await;

    assert!(matches!(probe_result, Err(CircuitBreakerError::Inner(_))));

    let final_result: Result<(), _> = circuit_breaker.execute(failing_operation).await;

    assert!(matches!(
        final_result,
        Err(CircuitBreakerError::CircuitOpen)
    ));
}
