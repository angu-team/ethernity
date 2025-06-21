use super::{BufferPool, SmartCache};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Gerenciador de memória para a workspace
pub struct MemoryManager {
    caches: RwLock<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>,
    buffer_pools: RwLock<HashMap<String, Arc<BufferPool>>>,
}

impl MemoryManager {
    /// Cria um novo gerenciador de memória
    pub fn new() -> Self {
        Self {
            caches: RwLock::new(HashMap::new()),
            buffer_pools: RwLock::new(HashMap::new()),
        }
    }

    /// Registra um cache
    pub fn register_cache<K, V>(&self, name: &str, cache: Arc<SmartCache<K, V>>)
    where
        K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        self.caches
            .write()
            .insert(name.to_string(), cache as Arc<dyn std::any::Any + Send + Sync>);
    }

    /// Registra um pool de buffers
    pub fn register_buffer_pool(&self, name: &str, pool: Arc<BufferPool>) {
        self.buffer_pools.write().insert(name.to_string(), pool);
    }

    /// Obtém estatísticas de uso de memória
    pub fn memory_usage(&self) -> MemoryUsageStats {
        let mut stats = MemoryUsageStats::default();

        // Coleta estatísticas de caches
        for (name, _cache) in self.caches.read().iter() {
            // Simplificado - apenas conta o número de caches
            stats.cache_stats.insert(
                name.clone(),
                CacheStatsInfo {
                    hits: 0,
                    misses: 0,
                    hit_ratio: 0.0,
                    entries: 0,
                },
            );
        }

        // Coleta estatísticas de pools de buffers
        for (name, pool) in self.buffer_pools.read().iter() {
            let pool_stats = pool.stats();
            stats.buffer_pool_stats.insert(
                name.clone(),
                BufferPoolStatsInfo {
                    allocations: pool_stats.allocations,
                    reuses: pool_stats.reuses,
                    reuse_ratio: if pool_stats.allocations > 0 {
                        pool_stats.reuses as f64 / (pool_stats.allocations + pool_stats.reuses) as f64
                    } else {
                        0.0
                    },
                },
            );
        }

        stats
    }
}

/// Estatísticas de uso de memória
#[derive(Debug, Default, Clone)]
pub struct MemoryUsageStats {
    pub cache_stats: HashMap<String, CacheStatsInfo>,
    pub buffer_pool_stats: HashMap<String, BufferPoolStatsInfo>,
}

/// Informações de estatísticas de cache
#[derive(Debug, Clone)]
pub struct CacheStatsInfo {
    pub hits: usize,
    pub misses: usize,
    pub hit_ratio: f64,
    pub entries: usize,
}

/// Informações de estatísticas de pool de buffers
#[derive(Debug, Clone)]
pub struct BufferPoolStatsInfo {
    pub allocations: usize,
    pub reuses: usize,
    pub reuse_ratio: f64,
}

