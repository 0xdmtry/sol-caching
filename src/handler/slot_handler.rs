use crate::{
    service::confirmation_service::{ConfirmationStatus, confirm_with_lru_and_breaker},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

pub async fn check_slot_confirmation_handler(
    State(app_state): State<AppState>,
    Path(slot): Path<u64>,
) -> impl IntoResponse {
    match confirm_with_lru_and_breaker(&app_state, slot).await {
        ConfirmationStatus::Confirmed => StatusCode::OK,
        ConfirmationStatus::NotConfirmed => StatusCode::NOT_FOUND,
        ConfirmationStatus::Error => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
