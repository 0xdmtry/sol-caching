use crate::state::AppState;

/// Retrieves all slots from the primary cache for recent slots
pub async fn get_all_latest_slots(app_state: &AppState) -> Vec<u64> {
    app_state.cache.get_all_slots().await
}

/// Retrieves all slots from the secondary LRU cache for on-demand lookups
pub async fn get_all_lru_slots(app_state: &AppState) -> Vec<u64> {
    app_state.lru_cache.get_all_slots().await
}
