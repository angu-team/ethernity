use ethernity_deeptrace::{BufferPool, MemoryManager, MemoryMonitor, SmartCache};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_smart_cache_basic_and_expiration() {
    let cache = Arc::new(SmartCache::<&'static str, i32>::new(1, Duration::from_millis(10)));
    cache.insert("a", 1);
    assert_eq!(cache.get(&"a"), Some(1));
    assert_eq!(cache.get(&"missing"), None);
    cache.insert("b", 2); // should evict "a"
    assert_eq!(cache.get(&"a"), None); // eviction counted as miss
    tokio::time::sleep(Duration::from_millis(15)).await;
    cache.insert("c", 3);
    assert_eq!(cache.get(&"c"), Some(3));
    tokio::time::sleep(Duration::from_millis(15)).await;
    assert_eq!(cache.get(&"c"), None); // expired
    let stats = cache.stats();
    assert_eq!(stats.inserts, 3);
    assert_eq!(stats.evictions, 0);
    assert_eq!(stats.hits, 2);
    // misses: "missing" and after eviction
    assert_eq!(stats.misses, 2);
    assert_eq!(stats.expirations, 1);
}

#[tokio::test]
async fn test_buffer_pool_usage() {
    let pool = Arc::new(BufferPool::new(4, 1));
    // first get allocates new
    let mut buf = pool.get_buffer();
    assert_eq!(buf.len(), 0);
    buf.extend_from_slice(&[1, 2]);
    pool.return_buffer(buf);
    // reuse existing
    let buf2 = pool.get_buffer();
    assert!(buf2.is_empty());
    pool.return_buffer(buf2);
    // returning again when full should not increase stats
    pool.return_buffer(vec![0u8; 4]);
    let stats = pool.stats();
    assert_eq!(stats.allocations, 1);
    assert_eq!(stats.reuses, 1);
    assert_eq!(stats.returns, 2);
    assert_eq!(stats.misses, 1);
}

#[tokio::test]
async fn test_memory_manager_and_monitor() {
    let manager = Arc::new(MemoryManager::new());
    let cache = Arc::new(SmartCache::<&'static str, i32>::new(1, Duration::from_millis(50)));
    manager.register_cache("cache", cache.clone());
    let pool = Arc::new(BufferPool::new(4, 1));
    pool.get_buffer(); // allocate once
    pool.return_buffer(vec![]);
    pool.get_buffer(); // reuse
    manager.register_buffer_pool("pool", pool.clone());

    let monitor = MemoryMonitor::new(manager.clone(), Duration::from_millis(5), 2);
    monitor.start_monitoring().await.unwrap();
    tokio::time::sleep(Duration::from_millis(20)).await;
    let history = monitor.get_history();
    assert!(!history.is_empty());
    assert!(history.len() <= 2);
    for snap in history.iter() {
        assert!(snap.system_memory.total_memory == 0);
        assert!(snap.stats.cache_stats.contains_key("cache"));
        assert!(snap.stats.buffer_pool_stats.contains_key("pool"));
    }
    let usage = manager.memory_usage();
    let pool_stats = usage.buffer_pool_stats.get("pool").unwrap();
    assert_eq!(pool_stats.allocations, 1);
    assert_eq!(pool_stats.reuses, 1);
    assert!((pool_stats.reuse_ratio - 0.5).abs() < f64::EPSILON);
}

