use crate::{
    handler::ping_handler::ping, handler::slot_handler::check_slot_confirmation_handler,
    state::AppState,
};
use axum::{Router, routing::get};

pub fn create_router(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(ping))
        .route(
            "/isSlotConfirmed/{slot}",
            get(check_slot_confirmation_handler),
        )
        .with_state(app_state)
}
