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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CallNode, CallTree, CallType, TokenTransfer, ContractCreation, ExecutionStep, TokenType, ContractType};
    use ethereum_types::{Address, U256};

    fn addr(n: u64) -> Address { Address::from_low_u64_be(n) }

    #[test]
    fn test_calculate_stats() {
        let child = CallNode {
            index:1, depth:1, call_type:CallType::Call,
            from: addr(1), to: Some(addr(2)), value: U256::zero(), gas: U256::zero(), gas_used: U256::zero(),
            input: vec![], output: vec![], error: Some("err".into()), children: vec![]
        };
        let root = CallNode {
            index:0, depth:0, call_type:CallType::Call,
            from: addr(0), to: Some(addr(1)), value: U256::zero(), gas: U256::zero(), gas_used: U256::zero(),
            input: vec![], output: vec![], error: None, children: vec![child.clone()]};
        let call_tree = CallTree{root};
        let result = TraceAnalysisResult{ call_tree, token_transfers: vec![TokenTransfer{token_type:TokenType::Erc20, token_address:addr(3), from:addr(0), to:addr(1), amount:U256::one(), token_id:None, call_index:0}], contract_creations: vec![ContractCreation{creator:addr(0), contract_address:addr(4), init_code:vec![], contract_type:ContractType::Unknown, call_index:0}], execution_path: vec![ExecutionStep{depth:0,call_type:CallType::Call,from:addr(0),to:addr(1),value:U256::zero(),input:vec![],output:vec![],gas_used:U256::one(),error:None}, ExecutionStep{depth:1,call_type:CallType::Call,from:addr(1),to:addr(2),value:U256::zero(),input:vec![],output:vec![],gas_used:U256::from(2u64),error:None}] };
        let stats = result.calculate_stats(42);
        assert_eq!(stats.total_calls, 2);
        assert_eq!(stats.failed_calls, 1);
        assert_eq!(stats.max_depth, 1);
        assert_eq!(stats.token_transfers, 1);
        assert_eq!(stats.contract_creations, 1);
        assert_eq!(stats.unique_addresses, 3);
        assert_eq!(stats.total_gas_used, U256::from(3u64));
        assert_eq!(stats.analysis_time_ms, 42);
    }
}
