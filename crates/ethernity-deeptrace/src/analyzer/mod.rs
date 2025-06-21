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
