/*!
 * Ethernity DeepTrace
 *
 * Biblioteca para análise profunda de transações EVM via call traces.
 * Permite rastreamento detalhado de fluxo de fundos, detecção de padrões
 * e análise de interações entre contratos.
 */

mod memory;
mod trace;
pub mod analyzer;
mod patterns;
mod utils;

use ethereum_types::{Address, H256, U256};
use std::sync::Arc;

pub use analyzer::*;
// Re-exportações públicas
pub use memory::*;
pub use patterns::*;
pub use trace::*;
pub use utils::*;

/// Configuração para análise de traces
#[derive(Debug, Clone)]
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

/// Configuração para detecção de padrões
#[derive(Debug, Clone)]
pub struct PatternDetectionConfig {
    /// Habilita detecção de padrões de token ERC20
    pub detect_erc20: bool,
}

impl Default for PatternDetectionConfig {
    fn default() -> Self {
        Self {
            detect_erc20: true,
        }
    }
}

/// Analisador de traces de transações
pub struct DeepTraceAnalyzer {
    config: TraceAnalysisConfig,
    rpc_client: Arc<dyn ethernity_core::traits::RpcProvider>,
    memory_manager: Arc<memory::MemoryManager>,
    pattern_detectors: Vec<Box<dyn PatternDetector>>,
}

impl DeepTraceAnalyzer {
    /// Cria um novo analisador de traces
    pub fn new(
        rpc_client: Arc<dyn ethernity_core::traits::RpcProvider>,
        config: Option<TraceAnalysisConfig>,
    ) -> Self {
        let config = config.unwrap_or_default();
        let memory_manager = Arc::new(memory::MemoryManager::new());

        // Inicializa os detectores de padrões
        let detectors_all: Vec<(bool, Box<dyn PatternDetector>)> = vec![
            (config.pattern_detection.detect_erc20, Box::new(Erc20PatternDetector::new())),
        ];
        
        let pattern_detectors: Vec<Box<dyn PatternDetector>> = detectors_all
            .into_iter()
            .filter_map(|(enabled, detector)| if enabled { Some(detector) } else { None })
            .collect();

        Self {
            config,
            rpc_client,
            memory_manager,
            pattern_detectors,
        }
    }

    /// Analisa uma transação pelo hash
    pub async fn analyze_transaction(&self, tx_hash: H256) -> Result<TransactionAnalysis, ()> {
        let trace = self.fetch_trace(tx_hash).await?;
        let receipt = self.fetch_receipt(tx_hash).await?;
        let (block_number, from, to, gas_used, status) = Self::parse_receipt_info(&receipt);
        let timestamp = chrono::Utc::now(); // Simplificado

        let context = AnalysisContext {
            tx_hash,
            block_number,
            timestamp,
            rpc_client: self.rpc_client.clone(),
            memory_manager: self.memory_manager.clone(),
            config: self.config.clone(),
        };

        let trace_analyzer = TraceAnalyzer::new(context);
        let analysis = trace_analyzer.analyze(&trace, &receipt).await.map_err(|_| ())?;
        let patterns = self.detect_patterns(&analysis).await?;

        Ok(Self::build_transaction_analysis(
            tx_hash,
            block_number,
            timestamp,
            from,
            to,
            gas_used,
            status,
            analysis,
            patterns,
        ))
    }

    async fn fetch_trace(&self, tx_hash: H256) -> Result<CallTrace, ()> {
        let bytes = self
            .rpc_client
            .get_transaction_trace(tx_hash)
            .await
            .map_err(|_| ())?;
        serde_json::from_slice(&bytes).map_err(|_| ())
    }

    async fn fetch_receipt(&self, tx_hash: H256) -> Result<serde_json::Value, ()> {
        let bytes = self
            .rpc_client
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|_| ())?;
        serde_json::from_slice(&bytes).map_err(|_| ())
    }

    fn parse_receipt_info(
        receipt: &serde_json::Value,
    ) -> (u64, Address, Option<Address>, U256, bool) {
        let block_number = receipt
            .get("blockNumber")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0);

        let from = receipt
            .get("from")
            .and_then(|v| v.as_str())
            .and_then(|s| {
                let addr_bytes = hex::decode(s.trim_start_matches("0x")).ok()?;
                if addr_bytes.len() >= 20 {
                    Some(Address::from_slice(&addr_bytes[addr_bytes.len() - 20..]))
                } else {
                    None
                }
            })
            .unwrap_or_else(Address::zero);

        let to = receipt
            .get("to")
            .and_then(|v| v.as_str())
            .and_then(|s| {
                let addr_bytes = hex::decode(s.trim_start_matches("0x")).ok()?;
                if addr_bytes.len() >= 20 {
                    Some(Address::from_slice(&addr_bytes[addr_bytes.len() - 20..]))
                } else {
                    None
                }
            });

        let gas_used = receipt
            .get("gasUsed")
            .and_then(|v| v.as_str())
            .and_then(|s| U256::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .unwrap_or_else(U256::zero);

        let status = receipt
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| s == "0x1")
            .unwrap_or(false);

        (block_number, from, to, gas_used, status)
    }

    fn build_transaction_analysis(
        tx_hash: H256,
        block_number: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
        from: Address,
        to: Option<Address>,
        gas_used: U256,
        status: bool,
        analysis: TraceAnalysisResult,
        patterns: Vec<DetectedPattern>,
    ) -> TransactionAnalysis {
        TransactionAnalysis {
            tx_hash,
            block_number,
            timestamp,
            from,
            to,
            value: U256::zero(), // Simplificado
            gas_used,
            status,
            call_tree: analysis.call_tree,
            token_transfers: analysis.token_transfers,
            contract_creations: analysis.contract_creations,
            detected_patterns: patterns,
            execution_path: analysis.execution_path,
        }
    }

    /// Detecta padrões na análise
    async fn detect_patterns(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        for detector in &self.pattern_detectors {
            let detected = detector.detect(analysis).await.map_err(|_| ())?;
            patterns.extend(detected);
        }

        Ok(patterns)
    }

    /// Analisa um lote de transações
    pub async fn analyze_batch(&self, tx_hashes: &[H256]) -> Result<Vec<TransactionAnalysis>, ()> {
        let mut results = Vec::with_capacity(tx_hashes.len());

        if self.config.enable_parallel {
            // Análise paralela
            let mut futures = Vec::with_capacity(tx_hashes.len());

            for &tx_hash in tx_hashes {
                futures.push(self.analyze_transaction(tx_hash));
            }

            let analyses = futures::future::join_all(futures).await;

            for analysis in analyses {
                match analysis {
                    Ok(result) => results.push(result),
                    Err(e) => eprintln!("Erro ao analisar transação: {:?}", e),
                }
            }
        } else {
            // Análise sequencial
            for &tx_hash in tx_hashes {
                match self.analyze_transaction(tx_hash).await {
                    Ok(result) => results.push(result),
                    Err(e) => eprintln!("Erro ao analisar transação: {:?}", e),
                }
            }
        }

        Ok(results)
    }

    /// Obtém estatísticas de uso de memória
    pub fn memory_stats(&self) -> memory::MemoryUsageStats {
        self.memory_manager.memory_usage()
    }
}

/// Resultado da análise de uma transação
#[derive(Debug)]
pub struct TransactionAnalysis {
    pub tx_hash: H256,
    pub block_number: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_used: U256,
    pub status: bool,
    pub call_tree: CallTree,
    pub token_transfers: Vec<TokenTransfer>,
    pub contract_creations: Vec<ContractCreation>,
    pub detected_patterns: Vec<DetectedPattern>,
    pub execution_path: Vec<ExecutionStep>,
}

/// Transferência de token
#[derive(Debug, Clone, PartialEq)]
pub struct TokenTransfer {
    pub token_type: TokenType,
    pub token_address: Address,
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    pub token_id: Option<U256>,
    pub call_index: usize,
}

/// Tipo de token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Erc20,
    Erc721,
    Erc1155,
    Unknown,
}

/// Criação de contrato
#[derive(Debug, Clone)]
pub struct ContractCreation {
    pub creator: Address,
    pub contract_address: Address,
    pub init_code: Vec<u8>,
    pub contract_type: ContractType,
    pub call_index: usize,
}

/// Tipo de contrato
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractType {
    Erc20Token,
    Erc721Token,
    Erc1155Token,
    DexPool,
    LendingPool,
    Proxy,
    Factory,
    Unknown,
}

/// Padrão detectado
#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub addresses: Vec<Address>,
    pub data: serde_json::Value,
    pub description: String,
}

/// Tipo de padrão
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    Erc20Creation,
    Unknown,
}

/// Passo de execução
#[derive(Debug, Clone)]
pub struct ExecutionStep {
    pub depth: usize,
    pub call_type: CallType,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub input: Vec<u8>,
    pub output: Vec<u8>,
    pub gas_used: U256,
    pub error: Option<String>,
}