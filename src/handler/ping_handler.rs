use axum::response::IntoResponse;

// Simple handler which could be used for healtchecks
// Response could contain timestamp, or any other additional metadata
// In current particular case, simple response would be more than enough
pub async fn ping() -> impl IntoResponse {
    "pong"
}
