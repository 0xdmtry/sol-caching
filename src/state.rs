use crate::{cache::SlotCache, rpc::RpcApi};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub rpc_client: Arc<dyn RpcApi + Send + Sync>,
    pub cache: Arc<SlotCache>,
}
