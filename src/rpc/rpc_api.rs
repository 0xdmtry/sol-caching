use solana_client::{client_error::ClientError, nonblocking::rpc_client::RpcClient};
use std::{future::Future, pin::Pin};

pub trait RpcApi: Send + Sync {
    fn get_slot<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<u64, ClientError>> + Send + 'a>>;

    fn get_blocks<'a>(
        &'a self,
        start_slot: u64,
        end_slot: Option<u64>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u64>, ClientError>> + Send + 'a>>;
}

impl RpcApi for RpcClient {
    fn get_slot<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<u64, ClientError>> + Send + 'a>> {
        Box::pin(self.get_slot())
    }

    fn get_blocks<'a>(
        &'a self,
        start_slot: u64,
        end_slot: Option<u64>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<u64>, ClientError>> + Send + 'a>> {
        Box::pin(self.get_blocks(start_slot, end_slot))
    }
}
