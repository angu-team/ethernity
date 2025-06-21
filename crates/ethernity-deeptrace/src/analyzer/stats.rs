use crate::TraceAnalysisResult;
use ethereum_types::U256;
use std::collections::HashSet;

#[derive(Debug)]
pub struct AnalysisStats {
    pub total_calls: usize,
    pub failed_calls: usize,
    pub max_depth: usize,
    pub token_transfers: usize,
    pub contract_creations: usize,
    pub unique_addresses: usize,
    pub total_gas_used: U256,
    pub analysis_time_ms: u64,
}

impl TraceAnalysisResult {
    pub fn calculate_stats(&self, analysis_time_ms: u64) -> AnalysisStats {
        let total_calls = self.call_tree.total_calls();
        let failed_calls = self.call_tree.failed_calls().len();
        let max_depth = self.call_tree.max_depth();
        let token_transfers = self.token_transfers.len();
        let contract_creations = self.contract_creations.len();
        let mut unique_addresses = HashSet::new();
        self.call_tree.traverse_preorder(|node| {
            unique_addresses.insert(node.from);
            if let Some(to) = node.to { unique_addresses.insert(to); }
        });
        let total_gas_used = self.execution_path.iter().map(|s| s.gas_used).fold(U256::zero(), |acc, g| acc + g);
        AnalysisStats {
            total_calls,
            failed_calls,
            max_depth,
            token_transfers,
            contract_creations,
            unique_addresses: unique_addresses.len(),
            total_gas_used,
            analysis_time_ms,
        }
    }
}
