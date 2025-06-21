use serde::{Deserialize, Serialize};

/// Configuração para detecção de padrões
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDetectionConfig {
    /// Habilita detecção de padrões de token ERC20
    pub detect_erc20: bool,
}

impl Default for PatternDetectionConfig {
    fn default() -> Self {
        Self { detect_erc20: true }
    }
}

/// Configuração para análise de traces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceAnalysisConfig {
    /// Profundidade máxima de análise recursiva
    pub max_depth: usize,
    /// Limite de memória em bytes
    pub memory_limit: usize,
    /// Timeout para análise em milissegundos
    pub timeout_ms: u64,
    /// Habilita cache de resultados intermediários
    pub enable_cache: bool,
    /// Habilita análise paralela quando possível
    pub enable_parallel: bool,
    /// Habilita detecção de padrões específicos
    pub pattern_detection: PatternDetectionConfig,
}

impl Default for TraceAnalysisConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            memory_limit: 100 * 1024 * 1024, // 100 MB
            timeout_ms: 30000, // 30 segundos
            enable_cache: true,
            enable_parallel: true,
            pattern_detection: PatternDetectionConfig::default(),
        }
    }
}
