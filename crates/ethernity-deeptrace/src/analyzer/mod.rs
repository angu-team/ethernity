// New modularized analyzer
mod call_tree;
mod token;
mod contracts;
mod execution;
mod stats;

pub use stats::AnalysisStats;

use call_tree::build_call_tree;
use contracts::extract_contract_creations;
use execution::build_execution_path;
use token::extract_token_transfers;

use crate::memory::MemoryManager;
use crate::{trace::*, ContractCreation, ExecutionStep, TokenTransfer, TraceAnalysisConfig};
use ethereum_types::{H256};
use std::sync::Arc;

pub struct AnalysisContext {
    pub tx_hash: H256,
    pub block_number: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub rpc_client: Arc<dyn ethernity_core::traits::RpcProvider>,
    pub memory_manager: Arc<MemoryManager>,
    pub config: TraceAnalysisConfig,
}

pub struct TraceAnalyzer {
    context: AnalysisContext,
}

impl TraceAnalyzer {
    pub fn new(context: AnalysisContext) -> Self {
        Self { context }
    }

    pub async fn analyze(
        &self,
        trace: &CallTrace,
        receipt: &serde_json::Value,
    ) -> Result<TraceAnalysisResult, ()> {
        let call_tree = build_call_tree(trace, &self.context.config)?;
        let token_transfers = extract_token_transfers(receipt).await?;
        let contract_creations = extract_contract_creations(self.context.rpc_client.clone(), trace).await?;
        let execution_path = build_execution_path(trace, &self.context.config)?;

        Ok(TraceAnalysisResult {
            call_tree,
            token_transfers,
            contract_creations,
            execution_path,
        })
    }
}

pub struct TraceAnalysisResult {
    pub call_tree: CallTree,
    pub token_transfers: Vec<TokenTransfer>,
    pub contract_creations: Vec<ContractCreation>,
    pub execution_path: Vec<ExecutionStep>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    struct MockRpc;

    #[async_trait]
    impl ethernity_core::traits::RpcProvider for MockRpc {
        async fn get_transaction_trace(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_transaction_receipt(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_code(&self, _address: ethereum_types::Address) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![0u8]) }
        async fn call(&self, _to: ethereum_types::Address, _data: Vec<u8>) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_block_number(&self) -> ethernity_core::error::Result<u64> { Ok(0) }
        async fn get_block_hash(&self, _block_number: u64) -> ethernity_core::error::Result<ethereum_types::H256> { Ok(ethereum_types::H256::zero()) }
    }

    fn simple_trace() -> CallTrace {
        CallTrace {
            from: "0x01".into(), gas: "0".into(), gas_used: "0".into(),
            to: "0x02".into(), input: "0x".into(), output: "0x".into(), value: "0".into(), error: None,
            calls: None, call_type: Some("CALL".into())
        }
    }

    #[tokio::test]
    async fn test_analyze_combines_components() {
        let ctx = AnalysisContext {
            tx_hash: H256::zero(),
            block_number: 0,
            timestamp: chrono::Utc::now(),
            rpc_client: Arc::new(MockRpc),
            memory_manager: Arc::new(MemoryManager::new()),
            config: TraceAnalysisConfig::default(),
        };
        let analyzer = TraceAnalyzer::new(ctx);
        let trace = simple_trace();
        let receipt = json!({"logs": []});
        let result = analyzer.analyze(&trace, &receipt).await.unwrap();
        assert_eq!(result.token_transfers.len(), 0);
        assert_eq!(result.contract_creations.len(), 0);
        assert_eq!(result.execution_path.len(), 1);
    }

    struct MockRpcSuccess;

    #[async_trait]
    impl ethernity_core::traits::RpcProvider for MockRpcSuccess {
        async fn get_transaction_trace(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_transaction_receipt(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_code(&self, _address: ethereum_types::Address) -> ethernity_core::error::Result<Vec<u8>> {
            Ok(vec![0x63,0x70,0xa0,0x82,0x31,0x00,0x00,0x63,0xa9,0x05,0x9c,0xbb,0x00,0x00])
        }
        async fn call(&self, _to: ethereum_types::Address, _data: Vec<u8>) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_block_number(&self) -> ethernity_core::error::Result<u64> { Ok(0) }
        async fn get_block_hash(&self, _block_number: u64) -> ethernity_core::error::Result<ethereum_types::H256> { Ok(ethereum_types::H256::zero()) }
    }

    struct MockRpcFail;

    #[async_trait]
    impl ethernity_core::traits::RpcProvider for MockRpcFail {
        async fn get_transaction_trace(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_transaction_receipt(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_code(&self, _address: ethereum_types::Address) -> ethernity_core::error::Result<Vec<u8>> {
            Err(ethernity_core::Error::Other("fail".into()))
        }
        async fn call(&self, _to: ethereum_types::Address, _data: Vec<u8>) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_block_number(&self) -> ethernity_core::error::Result<u64> { Ok(0) }
        async fn get_block_hash(&self, _block_number: u64) -> ethernity_core::error::Result<ethereum_types::H256> { Ok(ethereum_types::H256::zero()) }
    }

    fn creation_trace() -> CallTrace {
        CallTrace {
            from: "0x0000000000000000000000000000000000000001".into(),
            gas: "0".into(),
            gas_used: "0".into(),
            to: "0x0000000000000000000000000000000000000002".into(),
            input: "0x".into(),
            output: "0x".into(),
            value: "0".into(),
            error: None,
            calls: Some(vec![CallTrace {
                from: "0x0000000000000000000000000000000000000001".into(),
                gas: "0".into(),
                gas_used: "0".into(),
                to: "0x0000000000000000000000000000000000000100".into(),
                input: "0x".into(),
                output: "0x".into(),
                value: "0".into(),
                error: None,
                calls: None,
                call_type: Some("CREATE".into()),
            }]),
            call_type: Some("CALL".into()),
        }
    }

    #[tokio::test]
    async fn test_analyze_full_integration() {
        let ctx = AnalysisContext {
            tx_hash: H256::zero(),
            block_number: 0,
            timestamp: chrono::Utc::now(),
            rpc_client: Arc::new(MockRpcSuccess),
            memory_manager: Arc::new(MemoryManager::new()),
            config: TraceAnalysisConfig::default(),
        };
        let analyzer = TraceAnalyzer::new(ctx);
        let trace = creation_trace();
        let transfer_sig = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
        let receipt = json!({"logs": [{
            "address": "0x0000000000000000000000000000000000000001",
            "topics": [transfer_sig, "0x0000000000000000000000000000000000000002", "0x0000000000000000000000000000000000000003"],
            "data": "0x01"
        }]});
        let result = analyzer.analyze(&trace, &receipt).await.unwrap();
        assert_eq!(result.token_transfers.len(), 1);
        assert_eq!(result.contract_creations.len(), 1);
        assert_eq!(result.execution_path.len(), 2);
        assert_eq!(result.call_tree.root.call_type, CallType::Call);
    }

    #[tokio::test]
    async fn test_analyze_propagates_contract_creation_error() {
        let ctx = AnalysisContext {
            tx_hash: H256::zero(),
            block_number: 0,
            timestamp: chrono::Utc::now(),
            rpc_client: Arc::new(MockRpcFail),
            memory_manager: Arc::new(MemoryManager::new()),
            config: TraceAnalysisConfig::default(),
        };
        let analyzer = TraceAnalyzer::new(ctx);
        let trace = creation_trace();
        let receipt = json!({"logs": []});
        assert!(analyzer.analyze(&trace, &receipt).await.is_err());
    }

    #[test]
    fn test_new_stores_context() {
        let ctx = AnalysisContext {
            tx_hash: H256::from_low_u64_be(1),
            block_number: 1,
            timestamp: chrono::Utc::now(),
            rpc_client: Arc::new(MockRpc),
            memory_manager: Arc::new(MemoryManager::new()),
            config: TraceAnalysisConfig::default(),
        };
        let analyzer = TraceAnalyzer::new(ctx);
        assert_eq!(analyzer.context.block_number, 1);
    }
}
