use solana_caching_service::cache::LruCache;

#[tokio::test]
async fn test_lru_put_and_get() {
    let cache = LruCache::new(3);

    assert!(!cache.get(&100).await, "Cache should be empty");
    cache.put(100).await;
    assert!(
        cache.get(&100).await,
        "Cache should contain the key after put"
    );
    assert!(
        !cache.get(&200).await,
        "Cache should not contain a key that wasn't added"
    );
}

#[tokio::test]
async fn test_lru_eviction_logic() {
    let cache = LruCache::new(2);

    cache.put(1).await;
    cache.put(2).await;

    assert!(cache.get(&1).await);
    assert!(cache.get(&2).await);

    cache.put(3).await;

    assert!(!cache.get(&1).await, "Key 1 should have been evicted");
    assert!(cache.get(&2).await, "Key 2 should still exist");
    assert!(cache.get(&3).await, "Key 3 should have been added");
}

#[tokio::test]
async fn test_get_operation_updates_recency() {
    let cache = LruCache::new(3);
    cache.put(1).await;
    cache.put(2).await;
    cache.put(3).await;

    let _ = cache.get(&1).await;

    let slots = cache.get_all_slots().await;
    assert_eq!(
        slots,
        vec![1, 3, 2],
        "Getting a key should move it to the front"
    );

    cache.put(4).await;

    assert!(!cache.get(&2).await, "Key 2 should have been evicted");
    assert!(cache.get(&1).await);
    assert!(cache.get(&3).await);
    assert!(cache.get(&4).await);
}

#[tokio::test]
async fn test_put_operation_on_existing_key_updates_recency() {
    let cache = LruCache::new(3);
    cache.put(1).await;
    cache.put(2).await;
    cache.put(3).await;

    cache.put(1).await;

    let slots = cache.get_all_slots().await;
    assert_eq!(
        slots,
        vec![1, 3, 2],
        "Putting an existing key should move it to the front"
    );
}

#[tokio::test]
async fn test_get_all_slots_returns_correct_order() {
    let cache = LruCache::new(3);

    cache.put(1).await;
    cache.put(2).await;
    cache.put(3).await;

    assert_eq!(cache.get_all_slots().await, vec![3, 2, 1]);

    let _ = cache.get(&2).await;

    assert_eq!(cache.get_all_slots().await, vec![2, 3, 1]);
}
