use tokio::signal;
use tracing::info;

/// Listens for the shutdown signal (e.g., Ctrl+C) and returns a future
/// that resolves when the signal is received.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { info!("Received Ctrl+C, initiating graceful shutdown."); },
        _ = terminate => { info!("Received terminate signal, initiating graceful shutdown."); },
    }
}
