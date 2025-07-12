use solana_client::{client_error::ClientError, nonblocking::rpc_client::RpcClient};
use tracing::info;

/// Wrapper around RpcClient
pub struct SolanaRpcClient {
    client: RpcClient,
}

impl SolanaRpcClient {
    /// Creates a new SolanaRpcClient
    pub fn new(rpc_url: String) -> Self {
        info!("Init RPC client: {}", rpc_url);

        Self {
            client: RpcClient::new(rpc_url),
        }
    }

    /// Fetches the latest slot
    pub async fn get_latest_slot(&self) -> Result<u64, ClientError> {
        self.client.get_slot().await
    }

    /// Fetches the confirmed block numbers within a given slot range
    pub async fn get_confirmed_blocks(
        &self,
        start_slot: u64,
        end_slot: u64,
    ) -> Result<Vec<u64>, ClientError> {
        self.client.get_blocks(start_slot, Some(end_slot)).await
    }
}
