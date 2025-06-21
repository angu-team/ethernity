use ethernity_deeptrace::{
    CallTree, CallNode, CallType, TraceAnalysisResult,
    ContractCreation, ContractType, PatternType,
    Erc20PatternDetector, PatternDetector,
};
use ethereum_types::{Address, U256};
use serde_json::Value;

fn addr(n: u64) -> Address {
    Address::from_low_u64_be(n)
}

fn empty_analysis() -> TraceAnalysisResult {
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
async fn pattern_type_and_default_confidence() {
    let detector = Erc20PatternDetector::new();
    assert_eq!(detector.pattern_type(), PatternType::Erc20Creation);
    assert!((detector.min_confidence() - 0.7).abs() < f64::EPSILON);
}

#[tokio::test]
async fn detect_returns_empty_when_no_creations() {
    let analysis = empty_analysis();
    let detector = Erc20PatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert!(patterns.is_empty());
}

#[tokio::test]
async fn detect_skips_non_erc20_creations() {
    let mut analysis = empty_analysis();
    analysis.contract_creations = vec![
        ContractCreation {
            creator: addr(2),
            contract_address: addr(3),
            init_code: Vec::new(),
            contract_type: ContractType::Proxy,
            call_index: 0,
        },
        ContractCreation {
            creator: addr(4),
            contract_address: addr(5),
            init_code: Vec::new(),
            contract_type: ContractType::DexPool,
            call_index: 1,
        },
    ];

    let detector = Erc20PatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert!(patterns.is_empty());
}

#[tokio::test]
async fn detect_handles_multiple_and_mixed_creations() {
    let mut analysis = empty_analysis();
    analysis.contract_creations = vec![
        ContractCreation {
            creator: addr(10),
            contract_address: addr(100),
            init_code: Vec::new(),
            contract_type: ContractType::Erc20Token,
            call_index: 0,
        },
        ContractCreation {
            creator: addr(11),
            contract_address: addr(101),
            init_code: Vec::new(),
            contract_type: ContractType::Proxy,
            call_index: 1,
        },
        ContractCreation {
            creator: addr(12),
            contract_address: addr(102),
            init_code: Vec::new(),
            contract_type: ContractType::Erc20Token,
            call_index: 2,
        },
    ];

    let detector = Erc20PatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert_eq!(patterns.len(), 2);

    let first = &patterns[0];
    assert_eq!(first.pattern_type, PatternType::Erc20Creation);
    assert_eq!(first.confidence, 0.9);
    assert_eq!(first.addresses, vec![addr(100), addr(10)]);
    let data = first.data.as_object().expect("object");
    assert_eq!(data.get("contract_address"), Some(&Value::String(format!("{:?}", addr(100)))));
    assert_eq!(data.get("creator"), Some(&Value::String(format!("{:?}", addr(10)))));
    assert_eq!(first.description, "Criação de token ERC20 detectada");

    let second = &patterns[1];
    assert_eq!(second.addresses, vec![addr(102), addr(12)]);
}
