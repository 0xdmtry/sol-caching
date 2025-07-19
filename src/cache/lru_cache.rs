use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;

// Simplistic implementation of LRU,
// without doubly-linked list and without unsafe code
struct InnerLruCache {
    map: HashMap<u64, ()>,
    order: VecDeque<u64>,
    capacity: usize,
}

pub struct LruCache {
    inner: Mutex<InnerLruCache>,
}

impl InnerLruCache {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn move_to_front(&mut self, key: u64) {
        if let Some(pos) = self.order.iter().position(|&k| k == key) {
            self.order.remove(pos);
        }
        self.order.push_front(key);
    }
}

impl LruCache {
    pub fn new(capacity: usize) -> Self {
        let effective_capacity = if capacity == 0 { 1 } else { capacity };
        Self {
            inner: Mutex::new(InnerLruCache::new(effective_capacity)),
        }
    }

    pub async fn get(&self, key: &u64) -> bool {
        let mut inner = self.inner.lock().await;
        if inner.map.contains_key(key) {
            inner.move_to_front(*key);
            true
        } else {
            false
        }
    }

    pub async fn put(&self, key: u64) {
        let mut inner = self.inner.lock().await;

        if inner.map.contains_key(&key) {
            inner.move_to_front(key);
        } else {
            inner.order.push_front(key);
            inner.map.insert(key, ());

            if inner.order.len() > inner.capacity {
                if let Some(lru_key) = inner.order.pop_back() {
                    inner.map.remove(&lru_key);
                }
            }
        }
    }
}
