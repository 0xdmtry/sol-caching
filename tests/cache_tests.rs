use solana_caching_service::cache::SlotCache;

#[tokio::test]
async fn test_insert_and_contains() {
    let cache = SlotCache::new(5);
    assert!(!cache.contains(&100).await);
    cache.insert(100).await;
    assert!(cache.contains(&100).await);
}

#[tokio::test]
async fn test_get_latest_cached_slot() {
    let cache = SlotCache::new(5);
    assert_eq!(cache.get_latest_cached_slot().await, None);

    cache.insert(100).await;
    assert_eq!(cache.get_latest_cached_slot().await, Some(100));

    cache.insert(101).await;
    assert_eq!(cache.get_latest_cached_slot().await, Some(101));
}

#[tokio::test]
async fn test_eviction_logic() {
    let cache = SlotCache::new(3);

    cache.insert(1).await;
    cache.insert(2).await;
    cache.insert(3).await;

    assert!(cache.contains(&1).await);
    assert!(cache.contains(&2).await);
    assert!(cache.contains(&3).await);
    assert_eq!(cache.get_latest_cached_slot().await, Some(3));

    cache.insert(4).await;

    assert!(!cache.contains(&1).await, "Slot 1 should have been evicted");
    assert!(cache.contains(&2).await);
    assert!(cache.contains(&3).await);
    assert!(cache.contains(&4).await);
    assert_eq!(cache.get_latest_cached_slot().await, Some(4));

    cache.insert(5).await;

    assert!(!cache.contains(&2).await, "Slot 2 should have been evicted");
    assert!(cache.contains(&3).await);
    assert!(cache.contains(&4).await);
    assert!(cache.contains(&5).await);
}

#[tokio::test]
async fn test_no_duplicate_insertion() {
    let cache = SlotCache::new(3);

    cache.insert(1).await;
    cache.insert(2).await;
    cache.insert(1).await;
    cache.insert(2).await;
    cache.insert(3).await;

    assert!(cache.contains(&1).await);
    assert!(cache.contains(&2).await);
    assert!(cache.contains(&3).await);

    cache.insert(4).await;
    assert!(!cache.contains(&1).await);
    assert!(cache.contains(&2).await);
    assert!(cache.contains(&3).await);
    assert!(cache.contains(&4).await);
}
