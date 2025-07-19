use scc::HashMap;
use std::collections::VecDeque;
use tokio::sync::RwLock;
use tracing::trace;

#[derive(Debug)]
pub struct SlotCache {
    slots: HashMap<u64, ()>,
    order: RwLock<VecDeque<u64>>,
    capacity: usize,
}

impl SlotCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: HashMap::new(),
            order: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    pub async fn contains(&self, slot: &u64) -> bool {
        self.slots.contains(slot)
    }

    pub async fn get_latest_cached_slot(&self) -> Option<u64> {
        self.order.read().await.back().cloned()
    }

    pub async fn insert(&self, slot: u64) {
        if self.slots.insert(slot, ()).is_ok() {
            let mut order = self.order.write().await;
            order.push_back(slot);

            if order.len() > self.capacity {
                if let Some(oldest_slot) = order.pop_front() {
                    self.slots.remove(&oldest_slot);
                    trace!("Remove slot: {}", oldest_slot);
                }
            }
        }
    }

    pub async fn get_all_slots(&self) -> Vec<u64> {
        self.order.read().await.iter().cloned().collect()
    }
}
