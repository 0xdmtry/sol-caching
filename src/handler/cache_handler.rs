use crate::{
    service::cache_service::{get_all_latest_slots, get_all_lru_slots},
    state::AppState,
};
use axum::{extract::State, response::Json};

/// Handler to get all slots from the primary SlotCache
pub async fn get_latest_slots_handler(State(app_state): State<AppState>) -> Json<Vec<u64>> {
    Json(get_all_latest_slots(&app_state).await)
}

/// Handler to get all slots from the secondary LruCache
pub async fn get_lru_slots_handler(State(app_state): State<AppState>) -> Json<Vec<u64>> {
    Json(get_all_lru_slots(&app_state).await)
}
