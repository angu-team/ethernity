//! Módulo de gerenciamento de memória e performance

mod cache;
mod buffer_pool;
mod manager;
mod monitor;

pub use cache::{CacheStats, SmartCache};
pub use buffer_pool::{BufferPool, BufferPoolStats};
pub use manager::{BufferPoolStatsInfo, CacheStatsInfo, MemoryManager, MemoryUsageStats};
pub use monitor::{MemoryMonitor, MemoryUsageSnapshot, SystemMemoryInfo};

