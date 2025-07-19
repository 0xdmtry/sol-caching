use crate::{
    cache::{LruCache, SlotCache},
    circuit_breaker::CircuitBreaker,
    metrics::Metrics,
    rpc::RpcApi,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub rpc_client: Arc<dyn RpcApi + Send + Sync>,
    pub cache: Arc<SlotCache>,
    pub lru_cache: Arc<LruCache>,
    pub metrics: Arc<dyn Metrics + Send + Sync>,
    pub circuit_breaker: Arc<CircuitBreaker>,
}
