use super::rpc_api::RpcApi;
use solana_client::client_error::ClientError;
use tracing::info;

pub struct SolanaRpcClient<T: RpcApi> {
    client: T,
}

impl<T: RpcApi> SolanaRpcClient<T> {
    pub fn new(client: T) -> Self {
        info!("Initializing Solana RPC client");
        Self { client }
    }

    pub async fn get_latest_slot(&self) -> Result<u64, ClientError> {
        self.client.get_slot().await
    }

    pub async fn get_confirmed_blocks(
        &self,
        start_slot: u64,
        end_slot: Option<u64>,
    ) -> Result<Vec<u64>, ClientError> {
        self.client.get_blocks(start_slot, end_slot).await
    }
}