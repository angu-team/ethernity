use parking_lot::RwLock;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

/// Cache LRU para dados frequentemente acessados
pub struct SmartCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    cache: RwLock<lru::LruCache<K, CacheEntry<V>>>,
    stats: RwLock<CacheStats>,
    ttl: Duration,
}

/// Entrada de cache com timestamp
struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

/// Estatísticas de cache
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub inserts: usize,
    pub evictions: usize,
    pub expirations: usize,
}

impl<K, V> SmartCache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    /// Cria um novo cache com capacidade e TTL especificados
    pub fn new(capacity: usize, ttl: Duration) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or_else(|| NonZeroUsize::new(1).unwrap());

        Self {
            cache: RwLock::new(lru::LruCache::new(capacity)),
            stats: RwLock::new(CacheStats::default()),
            ttl,
        }
    }

    /// Obtém um valor do cache
    pub fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write();
        let now = Instant::now();

        if let Some(entry) = cache.get(key) {
            if entry.expires_at > now {
                // Cache hit
                self.stats.write().hits += 1;
                Some(entry.value.clone())
            } else {
                // Entry expired
                cache.pop(key);
                self.stats.write().expirations += 1;
                None
            }
        } else {
            // Cache miss
            self.stats.write().misses += 1;
            None
        }
    }

    /// Insere um valor no cache
    pub fn insert(&self, key: K, value: V) {
        let mut cache = self.cache.write();
        let expires_at = Instant::now() + self.ttl;

        if cache.put(key, CacheEntry { value, expires_at }).is_some() {
            self.stats.write().evictions += 1;
        }

        self.stats.write().inserts += 1;
    }

    /// Obtém estatísticas do cache
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }
}

