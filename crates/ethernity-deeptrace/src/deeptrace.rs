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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;
    use crate::{CallNode, CallTree, CallType, PatternType};

    struct MockRpc {
        trace: Vec<u8>,
        receipt: Vec<u8>,
        fail_trace: bool,
        fail_receipt: bool,
    }

    #[async_trait]
    impl ethernity_core::traits::RpcProvider for MockRpc {
        async fn get_transaction_trace(
            &self,
            _tx: ethernity_core::types::TransactionHash,
        ) -> ethernity_core::error::Result<Vec<u8>> {
            if self.fail_trace {
                Err(ethernity_core::Error::Other("fail".into()))
            } else {
                Ok(self.trace.clone())
            }
        }

        async fn get_transaction_receipt(
            &self,
            _tx: ethernity_core::types::TransactionHash,
        ) -> ethernity_core::error::Result<Vec<u8>> {
            if self.fail_receipt {
                Err(ethernity_core::Error::Other("fail".into()))
            } else {
                Ok(self.receipt.clone())
            }
        }

        async fn get_code(&self, _address: Address) -> ethernity_core::error::Result<Vec<u8>> {
            Ok(vec![])
        }

        async fn call(&self, _to: Address, _data: Vec<u8>) -> ethernity_core::error::Result<Vec<u8>> {
            Ok(vec![])
        }

        async fn get_block_number(&self) -> ethernity_core::error::Result<u64> {
            Ok(0)
        }

        async fn get_block_hash(&self, _block_number: u64) -> ethernity_core::error::Result<ethereum_types::H256> {
            Ok(ethereum_types::H256::zero())
        }
    }

    struct DummyDetector;

    #[async_trait]
    impl PatternDetector for DummyDetector {
        fn pattern_type(&self) -> PatternType { PatternType::Unknown }

        async fn detect(&self, _analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
            Ok(vec![DetectedPattern {
                pattern_type: PatternType::Unknown,
                confidence: 1.0,
                addresses: vec![],
                data: serde_json::Value::Null,
                description: "dummy".into(),
            }])
        }

        fn min_confidence(&self) -> f64 { 1.0 }
    }

    fn sample_trace_bytes() -> Vec<u8> {
        let trace = json!({
            "from": "0x0000000000000000000000000000000000000001",
            "gas": "0",
            "gasUsed": "0",
            "to": "0x0000000000000000000000000000000000000002",
            "input": "0x",
            "output": "0x",
            "value": "0",
            "error": null,
            "calls": null,
            "type": "CALL"
        });
        serde_json::to_vec(&trace).unwrap()
    }

    fn sample_receipt_bytes() -> Vec<u8> {
        let receipt = json!({
            "blockNumber": "0x10",
            "from": "0x0000000000000000000000000000000000000001",
            "to": "0x0000000000000000000000000000000000000002",
            "gasUsed": "0x20",
            "status": "0x1",
            "logs": []
        });
        serde_json::to_vec(&receipt).unwrap()
    }

    fn empty_analysis() -> TraceAnalysisResult {
        TraceAnalysisResult {
            call_tree: CallTree {
                root: CallNode {
                    index: 0,
                    depth: 0,
                    call_type: CallType::Call,
                    from: Address::zero(),
                    to: None,
                    value: U256::zero(),
                    gas: U256::zero(),
                    gas_used: U256::zero(),
                    input: Vec::new(),
                    output: Vec::new(),
                    error: None,
                    children: Vec::new(),
                },
            },
            token_transfers: Vec::new(),
            contract_creations: Vec::new(),
            execution_path: Vec::new(),
        }
    }

    #[tokio::test]
    async fn test_fetch_and_analyze_transaction() {
        let rpc = Arc::new(MockRpc {
            trace: sample_trace_bytes(),
            receipt: sample_receipt_bytes(),
            fail_trace: false,
            fail_receipt: false,
        });
        let mut config = TraceAnalysisConfig::default();
        config.enable_parallel = false;
        let mut analyzer = DeepTraceAnalyzer::new(rpc, Some(config));
        analyzer.pattern_detectors = vec![Box::new(DummyDetector)];
        let res = analyzer.analyze_transaction(H256::zero()).await.unwrap();
        assert_eq!(res.block_number, 16);
        assert_eq!(res.from, Address::from_low_u64_be(1));
        assert_eq!(res.to, Some(Address::from_low_u64_be(2)));
        assert_eq!(res.gas_used, U256::from(32u64));
        assert!(res.status);
        assert_eq!(res.detected_patterns.len(), 1);
    }

    #[tokio::test]
    async fn test_fetch_error_paths() {
        let rpc = Arc::new(MockRpc { trace: vec![], receipt: vec![], fail_trace: true, fail_receipt: true });
        let analyzer = DeepTraceAnalyzer::new(rpc, None);
        assert!(analyzer.fetch_trace(H256::zero()).await.is_err());
        assert!(analyzer.fetch_receipt(H256::zero()).await.is_err());
    }

    #[test]
    fn test_parse_and_build() {
        let receipt = json!({
            "blockNumber": "0x1",
            "from": "0x0000000000000000000000000000000000000003",
            "gasUsed": "0x5",
            "status": "0x0"
        });
        let (bn, from, to, gas, status) = DeepTraceAnalyzer::parse_receipt_info(&receipt);
        assert_eq!(bn, 1);
        assert_eq!(from, Address::from_low_u64_be(3));
        assert!(to.is_none());
        assert_eq!(gas, U256::from(5u64));
        assert!(!status);

        let analysis = empty_analysis();
        let tx = DeepTraceAnalyzer::build_transaction_analysis(
            H256::zero(),
            bn,
            chrono::Utc::now(),
            from,
            to,
            gas,
            status,
            analysis,
            Vec::new(),
        );
        assert_eq!(tx.block_number, 1);
        assert_eq!(tx.status, false);
    }

    #[tokio::test]
    async fn test_detect_patterns_directly() {
        let rpc = Arc::new(MockRpc { trace: vec![], receipt: vec![], fail_trace: false, fail_receipt: false });
        let analyzer = DeepTraceAnalyzer {
            config: TraceAnalysisConfig::default(),
            rpc_client: rpc,
            memory_manager: Arc::new(memory::MemoryManager::new()),
            pattern_detectors: vec![Box::new(DummyDetector)],
        };
        let patterns = analyzer.detect_patterns(&empty_analysis()).await.unwrap();
        assert_eq!(patterns.len(), 1);
    }

    #[tokio::test]
    async fn test_analyze_batch_parallel_and_sequential() {
        let rpc = Arc::new(MockRpc {
            trace: sample_trace_bytes(),
            receipt: sample_receipt_bytes(),
            fail_trace: false,
            fail_receipt: false,
        });

        let mut cfg = TraceAnalysisConfig::default();
        cfg.enable_parallel = false;
        let analyzer_seq = DeepTraceAnalyzer::new(rpc.clone(), Some(cfg.clone()));
        let hashes = vec![H256::zero(), H256::from_low_u64_be(1)];
        let res = analyzer_seq.analyze_batch(&hashes).await.unwrap();
        assert_eq!(res.len(), 2);

        cfg.enable_parallel = true;
        let analyzer_par = DeepTraceAnalyzer::new(rpc, Some(cfg));
        let res2 = analyzer_par.analyze_batch(&hashes).await.unwrap();
        assert_eq!(res2.len(), 2);
    }

    #[test]
    fn test_new_and_memory_stats() {
        let mut cfg = TraceAnalysisConfig::default();
        cfg.pattern_detection.detect_erc20 = false;
        let analyzer = DeepTraceAnalyzer::new(Arc::new(MockRpc { trace: vec![], receipt: vec![], fail_trace: false, fail_receipt: false }), Some(cfg));
        assert!(analyzer.pattern_detectors.is_empty());
        let stats = analyzer.memory_stats();
        assert!(stats.cache_stats.is_empty());
    }
}
