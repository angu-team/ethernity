use ethereum_types::{Address, H256, U256};
use std::sync::Arc;

use crate::{
    analyzer::{AnalysisContext, TraceAnalysisResult, TraceAnalyzer},
    config::TraceAnalysisConfig,
    memory,
    patterns::{Erc20PatternDetector, PatternDetector},
    trace::CallTrace,
    types::{DetectedPattern, TransactionAnalysis},
};

/// Analisador de traces de transações
pub struct DeepTraceAnalyzer {
    pub(crate) config: TraceAnalysisConfig,
    pub(crate) rpc_client: Arc<dyn ethernity_core::traits::RpcProvider>,
    pub(crate) memory_manager: Arc<memory::MemoryManager>,
    pub(crate) pattern_detectors: Vec<Box<dyn PatternDetector>>,
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
