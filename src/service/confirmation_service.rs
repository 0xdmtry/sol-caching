use crate::state::AppState;
use std::time::Instant;
use tracing::error;

#[derive(Debug, PartialEq)]
pub enum ConfirmationStatus {
    Confirmed,
    NotConfirmed,
    Error,
}

pub async fn check_slot_confirmation(app_state: &AppState, slot: u64) -> ConfirmationStatus {
    let now = Instant::now();

    let status = if app_state.cache.contains(&slot).await {
        ConfirmationStatus::Confirmed
    } else {
        match app_state.rpc_client.get_blocks(slot, Some(slot)).await {
            Ok(blocks) => {
                if blocks.contains(&slot) {
                    ConfirmationStatus::Confirmed
                } else {
                    ConfirmationStatus::NotConfirmed
                }
            }
            Err(e) => {
                error!("RPC error on fallback for slot {}: {}", slot, e);
                ConfirmationStatus::Error
            }
        }
    };

    app_state
        .metrics
        .record_is_slot_confirmed_elapsed(now.elapsed());

    status
}

pub async fn check_slot_confirmation_with_lru(
    app_state: &AppState,
    slot: u64,
) -> ConfirmationStatus {
    let now = Instant::now();

    let status = {
        if app_state.cache.contains(&slot).await {
            ConfirmationStatus::Confirmed
        } else if app_state.lru_cache.get(&slot).await {
            ConfirmationStatus::Confirmed
        } else {
            match app_state.rpc_client.get_blocks(slot, Some(slot)).await {
                Ok(blocks) => {
                    if blocks.contains(&slot) {
                        app_state.lru_cache.put(slot).await;
                        ConfirmationStatus::Confirmed
                    } else {
                        ConfirmationStatus::NotConfirmed
                    }
                }
                Err(e) => {
                    error!("RPC error during fallback check for slot {}: {}", slot, e);
                    ConfirmationStatus::Error
                }
            }
        }
    };

    app_state
        .metrics
        .record_is_slot_confirmed_elapsed(now.elapsed());
    status
}
