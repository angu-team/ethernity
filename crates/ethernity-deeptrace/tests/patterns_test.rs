use ethernity_deeptrace::{
    CallTree, CallNode, CallType, TraceAnalysisResult,
    ContractCreation, ContractType, PatternType,
    Erc20PatternDetector, PatternDetector
};
use ethereum_types::{Address, U256};

fn addr(n: u64) -> Address {
    Address::from_low_u64_be(n)
}

fn basic_analysis() -> TraceAnalysisResult {
    TraceAnalysisResult {
        call_tree: CallTree {
            root: CallNode {
                index: 0,
                depth: 0,
                call_type: CallType::Call,
                from: addr(0),
                to: Some(addr(1)),
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
async fn test_erc20_creation_detection() {
    let mut analysis = basic_analysis();
    analysis.contract_creations = vec![ContractCreation {
        creator: addr(2),
        contract_address: addr(100),
        init_code: Vec::new(),
        contract_type: ContractType::Erc20Token,
        call_index: 0,
    }];

    let detector = Erc20PatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].pattern_type, PatternType::Erc20Creation);
}
