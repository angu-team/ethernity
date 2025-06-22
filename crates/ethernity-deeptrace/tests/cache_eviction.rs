use ethernity_deeptrace::SmartCache;
use std::time::Duration;

#[tokio::test]
async fn test_smart_cache_overwrite_counts_eviction() {
    let cache = SmartCache::<&'static str, i32>::new(2, Duration::from_millis(50));

    cache.insert("x", 1);
    assert_eq!(cache.get(&"x"), Some(1));

    // overwriting same key should increment eviction counter
    cache.insert("x", 2);
    assert_eq!(cache.get(&"x"), Some(2));

    let stats = cache.stats();
    assert_eq!(stats.inserts, 2);
    assert_eq!(stats.evictions, 1);
    // two successful gets above
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.expirations, 0);
}
