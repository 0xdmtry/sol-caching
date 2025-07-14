use std::{
    error::Error,
    fmt,
    future::Future,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::warn;

#[derive(Debug, Clone, Copy)]
enum State {
    Closed,
    Open { until: Instant },
    HalfOpen,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    state: RwLock<State>,
    failure_threshold: u32,
    open_duration: Duration,
    consecutive_failures: AtomicU32,
}

#[derive(Debug)]
pub enum CircuitBreakerError<E> {
    CircuitOpen,
    Inner(E),
}

impl<E: fmt::Debug> fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit is open"),
            CircuitBreakerError::Inner(e) => write!(f, "Inner operation failed: {:?}", e),
        }
    }
}

impl<E: Error + 'static> Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CircuitBreakerError::CircuitOpen => None,
            CircuitBreakerError::Inner(e) => Some(e),
        }
    }
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, open_duration: Duration) -> Self {
        Self {
            state: RwLock::new(State::Closed),
            failure_threshold,
            open_duration,
            consecutive_failures: AtomicU32::new(0),
        }
    }

    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        match self.check_state().await {
            State::Closed | State::HalfOpen => {
                let result = operation().await;
                self.handle_result(&result).await;
                result.map_err(CircuitBreakerError::Inner)
            }
            State::Open { .. } => Err(CircuitBreakerError::CircuitOpen),
        }
    }

    async fn check_state(&self) -> State {
        let mut state = self.state.write().await;
        if let State::Open { until } = *state {
            if Instant::now() >= until {
                warn!("Circuit breaker transitioning to HalfOpen state.");
                *state = State::HalfOpen;
            }
        }
        *state
    }

    async fn handle_result<T, E>(&self, result: &Result<T, E>) {
        let mut state = self.state.write().await;
        match result {
            Ok(_) => match *state {
                State::HalfOpen => {
                    warn!("Circuit breaker transitioning to Closed state after successful probe.");
                    *state = State::Closed;
                    self.consecutive_failures.store(0, Ordering::SeqCst);
                }
                State::Closed => {
                    self.consecutive_failures.store(0, Ordering::SeqCst);
                }
                _ => {}
            },
            Err(_) => match *state {
                State::HalfOpen => {
                    warn!("Circuit breaker probe failed, transitioning back to Open.");
                    *state = State::Open {
                        until: Instant::now() + self.open_duration,
                    };
                }
                State::Closed => {
                    let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
                    if failures >= self.failure_threshold {
                        warn!("Failure threshold reached. Circuit breaker transitioning to Open.");
                        *state = State::Open {
                            until: Instant::now() + self.open_duration,
                        };
                    }
                }
                _ => {}
            },
        }
    }
}
