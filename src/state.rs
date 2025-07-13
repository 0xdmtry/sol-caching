use crate::{cache::SlotCache, rpc::SolanaRpcClient};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub rpc_client: Arc<SolanaRpcClient<RpcClient>>,
    pub cache: Arc<SlotCache>,
}
