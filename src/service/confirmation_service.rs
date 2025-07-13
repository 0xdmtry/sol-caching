use crate::state::AppState;
use tracing::error;

#[derive(Debug, PartialEq)]
pub enum ConfirmationStatus {
    Confirmed,
    NotConfirmed,
    Error,
}

pub async fn check_slot_confirmation(app_state: &AppState, slot: u64) -> ConfirmationStatus {
    if app_state.cache.contains(&slot).await {
        return ConfirmationStatus::Confirmed;
    }

    match app_state
        .rpc_client
        .get_confirmed_blocks(slot, Some(slot))
        .await
    {
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
}
