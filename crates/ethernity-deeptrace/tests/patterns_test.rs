use ethernity_deeptrace::{
    CallTree, CallNode, CallType, TraceAnalysisResult, TokenTransfer, TokenType,
    ContractCreation, ContractType, PatternType,
    Erc20PatternDetector, Erc721PatternDetector, DexPatternDetector,
    LendingPatternDetector, FlashLoanPatternDetector, MevPatternDetector,
    RugPullPatternDetector, GovernancePatternDetector,
    PatternDetector
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

#[tokio::test]
async fn test_erc721_creation_detection() {
    let mut analysis = basic_analysis();
    analysis.contract_creations = vec![ContractCreation {
        creator: addr(3),
        contract_address: addr(101),
        init_code: Vec::new(),
        contract_type: ContractType::Erc721Token,
        call_index: 0,
    }];

    let detector = Erc721PatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].pattern_type, PatternType::Erc721Creation);
}

#[tokio::test]
async fn test_dex_swap_detection() {
    let mut analysis = basic_analysis();
    let token_a = addr(10);
    let token_b = addr(11);

    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token_a, from: addr(1), to: addr(2), amount: U256::from(100u64), token_id: None, call_index: 0 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token_b, from: addr(2), to: addr(1), amount: U256::from(200u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token_a, from: addr(2), to: addr(3), amount: U256::from(50u64), token_id: None, call_index: 2 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token_b, from: addr(3), to: addr(2), amount: U256::from(60u64), token_id: None, call_index: 3 },
    ];

    let detector = DexPatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].pattern_type, PatternType::TokenSwap);
}

#[tokio::test]
async fn test_lending_pattern_detection() {
    let mut analysis = basic_analysis();
    let token = addr(20);

    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(4), to: addr(5), amount: U256::from(1000u64), token_id: None, call_index: 0 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(5), to: addr(4), amount: U256::from(100u64), token_id: None, call_index: 1 },
    ];

    let detector = LendingPatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].pattern_type, PatternType::Liquidity);
}

#[tokio::test]
async fn test_flash_loan_detection() {
    let mut analysis = basic_analysis();
    let token = addr(30);

    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(6), to: addr(7), amount: U256::from(100000u64), token_id: None, call_index: 0 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(7), to: addr(8), amount: U256::from(1u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(7), to: addr(6), amount: U256::from(100050u64), token_id: None, call_index: 2 },
    ];

    let detector = FlashLoanPatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].pattern_type, PatternType::FlashLoan);
}

#[tokio::test]
async fn test_mev_arbitrage_detection() {
    let mut analysis = basic_analysis();
    let token = addr(40);

    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(9), to: addr(10), amount: U256::from(2000u64), token_id: None, call_index: 0 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(10), to: addr(11), amount: U256::from(2000u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(11), to: addr(9), amount: U256::from(5000u64), token_id: None, call_index: 2 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(9), to: addr(12), amount: U256::from(500u64), token_id: None, call_index: 3 },
    ];

    let detector = MevPatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].pattern_type, PatternType::Arbitrage);
}

#[tokio::test]
async fn test_rug_pull_detection() {
    let mut analysis = basic_analysis();
    let token = addr(50);
    let creator = addr(51);
    analysis.contract_creations = vec![ContractCreation {
        creator,
        contract_address: token,
        init_code: Vec::new(),
        contract_type: ContractType::Erc20Token,
        call_index: 0,
    }];
    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(60), to: creator, amount: U256::from(800000u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(61), to: creator, amount: U256::from(900000u64), token_id: None, call_index: 2 },
    ];

    let detector = RugPullPatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].pattern_type, PatternType::RugPull);
}

#[tokio::test]
async fn test_governance_activity_detection() {
    let mut analysis = basic_analysis();
    analysis.call_tree.root.input = vec![0xda, 0x35, 0xc6, 0x64];

    let detector = GovernancePatternDetector::new();
    let patterns = detector.detect(&analysis).await.unwrap();
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].pattern_type, PatternType::Governance);
}
