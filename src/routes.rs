use crate::{
    handler::cache_handler::{get_latest_slots_handler, get_lru_slots_handler},
    handler::ping_handler::ping,
    handler::slot_handler::check_slot_confirmation_handler,
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
        .route("/cache/latest", get(get_latest_slots_handler))
        .route("/cache/lru", get(get_lru_slots_handler))
        .with_state(app_state)
}
