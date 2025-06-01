/*!
 * Ethernity DeepTrace
 * 
 * Biblioteca para análise profunda de transações EVM via call traces.
 * Permite rastreamento detalhado de fluxo de fundos, detecção de padrões
 * e análise de interações entre contratos.
 */

mod memory;
mod trace;
mod analyzer;
mod patterns;
mod detectors;
mod utils;

use ethernity_core::{Error, Result, types::*};
use std::sync::Arc;

// Re-exportações públicas
pub use memory::*;
pub use trace::*;
pub use analyzer::*;
pub use patterns::*;
pub use detectors::*;
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
    /// Habilita detecção de padrões de token ERC721
    pub detect_erc721: bool,
    /// Habilita detecção de padrões de DEX
    pub detect_dex: bool,
    /// Habilita detecção de padrões de lending
    pub detect_lending: bool,
    /// Habilita detecção de padrões de flash loan
    pub detect_flash_loan: bool,
    /// Habilita detecção de padrões de MEV
    pub detect_mev: bool,
    /// Habilita detecção de padrões de rug pull
    pub detect_rug_pull: bool,
    /// Habilita detecção de padrões de governança
    pub detect_governance: bool,
}

impl Default for PatternDetectionConfig {
    fn default() -> Self {
        Self {
            detect_erc20: true,
            detect_erc721: true,
            detect_dex: true,
            detect_lending: true,
            detect_flash_loan: true,
            detect_mev: true,
            detect_rug_pull: true,
            detect_governance: true,
        }
    }
}

/// Analisador de traces de transações
pub struct DeepTraceAnalyzer {
    config: TraceAnalysisConfig,
    rpc_client: Arc<dyn ethernity_core::traits::RpcProvider>,
    memory_manager: Arc<memory::MemoryManager>,
    pattern_detectors: Vec<Box<dyn patterns::PatternDetector>>,
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
        let mut pattern_detectors: Vec<Box<dyn patterns::PatternDetector>> = Vec::new();
        
        if config.pattern_detection.detect_erc20 {
            pattern_detectors.push(Box::new(patterns::Erc20PatternDetector::new()));
        }
        
        if config.pattern_detection.detect_erc721 {
            pattern_detectors.push(Box::new(patterns::Erc721PatternDetector::new()));
        }
        
        if config.pattern_detection.detect_dex {
            pattern_detectors.push(Box::new(patterns::DexPatternDetector::new()));
        }
        
        if config.pattern_detection.detect_lending {
            pattern_detectors.push(Box::new(patterns::LendingPatternDetector::new()));
        }
        
        if config.pattern_detection.detect_flash_loan {
            pattern_detectors.push(Box::new(patterns::FlashLoanPatternDetector::new()));
        }
        
        if config.pattern_detection.detect_mev {
            pattern_detectors.push(Box::new(patterns::MevPatternDetector::new()));
        }
        
        if config.pattern_detection.detect_rug_pull {
            pattern_detectors.push(Box::new(patterns::RugPullPatternDetector::new()));
        }
        
        if config.pattern_detection.detect_governance {
            pattern_detectors.push(Box::new(patterns::GovernancePatternDetector::new()));
        }
        
        Self {
            config,
            rpc_client,
            memory_manager,
            pattern_detectors,
        }
    }
    
    /// Analisa uma transação pelo hash
    pub async fn analyze_transaction(&self, tx_hash: H256) -> Result<TransactionAnalysis> {
        // Obtém o trace da transação
        let trace_bytes = self.rpc_client.get_transaction_trace(tx_hash).await?;
        
        // Deserializa o trace
        let trace: trace::CallTrace = serde_json::from_slice(&trace_bytes)
            .map_err(|e| Error::DecodeError(format!("Falha ao deserializar trace: {}", e)))?;
        
        // Obtém o recibo da transação
        let receipt_bytes = self.rpc_client.get_transaction_receipt(tx_hash).await?;
        
        // Deserializa o recibo
        let receipt: web3::types::TransactionReceipt = serde_json::from_slice(&receipt_bytes)
            .map_err(|e| Error::DecodeError(format!("Falha ao deserializar recibo: {}", e)))?;
        
        // Obtém o bloco para obter o timestamp
        let block_number = receipt.block_number.unwrap_or_default();
        let block = self.rpc_client.call(
            Address::zero(),
            ethabi::encode(&[ethabi::Token::Uint(block_number)])
        ).await?;
        
        // Extrai o timestamp do bloco
        let block_data: web3::types::Block<web3::types::H256> = serde_json::from_slice(&block)
            .map_err(|e| Error::DecodeError(format!("Falha ao deserializar bloco: {}", e)))?;
        
        let timestamp = chrono::DateTime::from_timestamp(
            block_data.timestamp.as_u64() as i64, 
            0
        ).unwrap_or_else(|| chrono::Utc::now());
        
        // Cria o contexto de análise
        let context = analyzer::AnalysisContext {
            tx_hash,
            block_number: receipt.block_number.unwrap_or_default().as_u64(),
            timestamp,
            rpc_client: self.rpc_client.clone(),
            memory_manager: self.memory_manager.clone(),
            config: self.config.clone(),
        };
        
        // Cria o analisador
        let trace_analyzer = analyzer::TraceAnalyzer::new(context);
        
        // Analisa o trace
        let analysis = trace_analyzer.analyze(&trace, &receipt).await?;
        
        // Detecta padrões
        let patterns = self.detect_patterns(&analysis).await?;
        
        // Cria a análise final
        let transaction_analysis = TransactionAnalysis {
            tx_hash,
            block_number: receipt.block_number.unwrap_or_default().as_u64(),
            timestamp,
            from: Address::from_slice(receipt.from.as_bytes()),
            to: receipt.to.map(|addr| Address::from_slice(addr.as_bytes())),
            value: U256::from_big_endian(&receipt.value.unwrap_or_default().to_be_bytes::<32>()),
            gas_used: U256::from_big_endian(&receipt.gas_used.unwrap_or_default().to_be_bytes::<32>()),
            status: receipt.status.unwrap_or_default().as_u64() == 1,
            call_tree: analysis.call_tree,
            token_transfers: analysis.token_transfers,
            contract_creations: analysis.contract_creations,
            detected_patterns: patterns,
            execution_path: analysis.execution_path,
        };
        
        Ok(transaction_analysis)
    }
    
    /// Detecta padrões na análise
    async fn detect_patterns(&self, analysis: &analyzer::TraceAnalysisResult) -> Result<Vec<DetectedPattern>> {
        let mut patterns = Vec::new();
        
        for detector in &self.pattern_detectors {
            let detected = detector.detect(analysis).await?;
            patterns.extend(detected);
        }
        
        Ok(patterns)
    }
    
    /// Analisa um lote de transações
    pub async fn analyze_batch(&self, tx_hashes: &[H256]) -> Result<Vec<TransactionAnalysis>> {
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
                    Err(e) => eprintln!("Erro ao analisar transação: {}", e),
                }
            }
        } else {
            // Análise sequencial
            for &tx_hash in tx_hashes {
                match self.analyze_transaction(tx_hash).await {
                    Ok(result) => results.push(result),
                    Err(e) => eprintln!("Erro ao analisar transação: {}", e),
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
    pub call_tree: trace::CallTree,
    pub token_transfers: Vec<TokenTransfer>,
    pub contract_creations: Vec<ContractCreation>,
    pub detected_patterns: Vec<DetectedPattern>,
    pub execution_path: Vec<ExecutionStep>,
}

/// Transferência de token
#[derive(Debug, Clone)]
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
    Erc721Creation,
    TokenSwap,
    Liquidity,
    FlashLoan,
    Arbitrage,
    Frontrunning,
    Backrunning,
    Sandwich,
    RugPull,
    Governance,
    Unknown,
}

/// Passo de execução
#[derive(Debug, Clone)]
pub struct ExecutionStep {
    pub depth: usize,
    pub call_type: trace::CallType,
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub input: Vec<u8>,
    pub output: Vec<u8>,
    pub gas_used: U256,
    pub error: Option<String>,
}
