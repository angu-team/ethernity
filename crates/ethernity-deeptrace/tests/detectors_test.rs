use ethernity_deeptrace::{
    CallTree, CallNode, CallType, TraceAnalysisResult, TokenTransfer, TokenType,
    SandwichAttackDetector, FrontrunningDetector, ReentrancyDetector,
    PriceManipulationDetector, SuspiciousLiquidationDetector,
};
use ethernity_deeptrace::SpecializedDetector;
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

fn nested_analysis() -> TraceAnalysisResult {
    let grandchild = CallNode {
        index: 2,
        depth: 2,
        call_type: CallType::Call,
        from: addr(2),
        to: Some(addr(3)),
        value: U256::zero(),
        gas: U256::zero(),
        gas_used: U256::zero(),
        input: Vec::new(),
        output: Vec::new(),
        error: None,
        children: Vec::new(),
    };

    let child = CallNode {
        index: 1,
        depth: 1,
        call_type: CallType::Call,
        from: addr(1),
        to: Some(addr(2)),
        value: U256::zero(),
        gas: U256::zero(),
        gas_used: U256::zero(),
        input: Vec::new(),
        output: Vec::new(),
        error: None,
        children: vec![grandchild],
    };

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
                children: vec![child],
            },
        },
        token_transfers: Vec::new(),
        contract_creations: Vec::new(),
        execution_path: Vec::new(),
    }
}

#[tokio::test]
async fn test_sandwich_attack_detection() {
    let mut analysis = basic_analysis();
    let token = addr(10);
    let attacker = addr(11);
    let victim = addr(12);
    let dex = addr(13);

    let other = addr(14);
    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: attacker, to: dex, amount: U256::from(100u64), token_id: None, call_index: 0 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: victim, to: other, amount: U256::from(50u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: dex, to: attacker, amount: U256::from(120u64), token_id: None, call_index: 2 },
    ];

    let detector = SandwichAttackDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "sandwich_attack");
    assert!(events[0].addresses.contains(&attacker));
    assert!(events[0].addresses.contains(&victim));
}

#[tokio::test]
async fn test_frontrunning_detection() {
    // Build call tree with two sequential calls
    let mut root = CallNode {
        index: 0,
        depth: 0,
        call_type: CallType::Call,
        from: addr(1),
        to: Some(addr(20)),
        value: U256::zero(),
        gas: U256::zero(),
        gas_used: U256::zero(),
        input: Vec::new(),
        output: Vec::new(),
        error: None,
        children: Vec::new(),
    };

    let call1 = CallNode {
        index: 1,
        depth: 1,
        call_type: CallType::Call,
        from: addr(2),
        to: Some(addr(30)),
        value: U256::zero(),
        gas: U256::zero(),
        gas_used: U256::zero(),
        input: vec![1,2,3,4],
        output: Vec::new(),
        error: None,
        children: Vec::new(),
    };
    let call2 = CallNode {
        index: 2,
        depth: 1,
        call_type: CallType::Call,
        from: addr(3),
        to: Some(addr(30)),
        value: U256::zero(),
        gas: U256::zero(),
        gas_used: U256::zero(),
        input: vec![1,2,3,4,5],
        output: Vec::new(),
        error: None,
        children: Vec::new(),
    };
    let mut root_children = Vec::new();
    root_children.push(call1.clone());
    root_children.push(call2.clone());
    root.children = root_children;

    let analysis = TraceAnalysisResult {
        call_tree: CallTree { root },
        token_transfers: Vec::new(),
        contract_creations: Vec::new(),
        execution_path: Vec::new(),
    };

    let detector = FrontrunningDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "frontrunning");
}

#[tokio::test]
async fn test_reentrancy_detection() {
    // Construct call pattern A->B -> B->A -> A->B -> B->A
    let mut node0 = CallNode {
        index: 0,
        depth: 0,
        call_type: CallType::Call,
        from: addr(1),
        to: Some(addr(2)),
        value: U256::zero(),
        gas: U256::zero(),
        gas_used: U256::zero(),
        input: Vec::new(),
        output: Vec::new(),
        error: None,
        children: Vec::new(),
    };

    let mut node1 = CallNode {
        index: 1,
        depth: 1,
        call_type: CallType::Call,
        from: addr(2),
        to: Some(addr(1)),
        value: U256::zero(),
        gas: U256::zero(),
        gas_used: U256::zero(),
        input: Vec::new(),
        output: Vec::new(),
        error: None,
        children: Vec::new(),
    };

    let mut node2 = CallNode {
        index: 2,
        depth: 2,
        call_type: CallType::Call,
        from: addr(1),
        to: Some(addr(2)),
        value: U256::zero(),
        gas: U256::zero(),
        gas_used: U256::zero(),
        input: Vec::new(),
        output: Vec::new(),
        error: None,
        children: Vec::new(),
    };

    let node3 = CallNode {
        index: 3,
        depth: 3,
        call_type: CallType::Call,
        from: addr(2),
        to: Some(addr(1)),
        value: U256::zero(),
        gas: U256::zero(),
        gas_used: U256::zero(),
        input: Vec::new(),
        output: Vec::new(),
        error: None,
        children: Vec::new(),
    };

    node2.children.push(node3);
    node1.children.push(node2);
    node0.children.push(node1);

    let analysis = TraceAnalysisResult {
        call_tree: CallTree { root: node0 },
        token_transfers: Vec::new(),
        contract_creations: Vec::new(),
        execution_path: Vec::new(),
    };

    let detector = ReentrancyDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "reentrancy");
}

#[tokio::test]
async fn test_price_manipulation_detection() {
    let mut analysis = basic_analysis();
    let token = addr(5);
    let manipulator = addr(6);
    let pool = addr(7);
    let other = addr(8);

    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: manipulator, to: pool, amount: U256::from(2_000_000u64), token_id: None, call_index: 0 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: pool, to: other, amount: U256::from(100u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: other, to: manipulator, amount: U256::from(50u64), token_id: None, call_index: 2 },
    ];

    let detector = PriceManipulationDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "price_manipulation");
}

#[tokio::test]
async fn test_suspicious_liquidation_detection() {
    let mut analysis = basic_analysis();
    let token = addr(40);
    let manipulator = addr(41);
    let victim = addr(42);
    let pool = addr(43);

    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: manipulator, to: pool, amount: U256::from(200_000u64), token_id: None, call_index: 0 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: victim, to: pool, amount: U256::from(50u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: pool, to: manipulator, amount: U256::from(150u64), token_id: None, call_index: 2 },
    ];

    let detector = SuspiciousLiquidationDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "suspicious_liquidation");
}

#[tokio::test]
async fn test_sandwich_attack_detection_internal() {
    let mut analysis = nested_analysis();
    let token = addr(20);
    let attacker = addr(21);
    let victim = addr(22);
    let dex = addr(23);

    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: attacker, to: dex, amount: U256::from(100u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: victim, to: addr(24), amount: U256::from(50u64), token_id: None, call_index: 2 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: dex, to: attacker, amount: U256::from(130u64), token_id: None, call_index: 3 },
    ];

    let detector = SandwichAttackDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "sandwich_attack");
}

#[tokio::test]
async fn test_frontrunning_detection_internal() {
    let mut analysis = nested_analysis();
    // add two calls at depth 2 sequentially
    if let Some(child) = analysis.call_tree.root.children.get_mut(0) {
        child.children = vec![
            CallNode {
                index: 2,
                depth: 2,
                call_type: CallType::Call,
                from: addr(30),
                to: Some(addr(40)),
                value: U256::zero(),
                gas: U256::zero(),
                gas_used: U256::zero(),
                input: vec![0xaa, 0xbb, 0xcc, 0xdd, 1],
                output: Vec::new(),
                error: None,
                children: Vec::new(),
            },
            CallNode {
                index: 3,
                depth: 2,
                call_type: CallType::Call,
                from: addr(31),
                to: Some(addr(40)),
                value: U256::zero(),
                gas: U256::zero(),
                gas_used: U256::zero(),
                input: vec![0xaa, 0xbb, 0xcc, 0xdd, 2],
                output: Vec::new(),
                error: None,
                children: Vec::new(),
            },
        ];
    }

    let detector = FrontrunningDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "frontrunning");
}

#[tokio::test]
async fn test_reentrancy_detection_internal() {
    let mut analysis = nested_analysis();
    // Build call sequence at depth 2 causing reentrancy
    if let Some(child) = analysis.call_tree.root.children.get_mut(0) {
        let mut first = CallNode {
            index: 2,
            depth: 2,
            call_type: CallType::Call,
            from: addr(50),
            to: Some(addr(60)),
            value: U256::zero(),
            gas: U256::zero(),
            gas_used: U256::zero(),
            input: Vec::new(),
            output: Vec::new(),
            error: None,
            children: Vec::new(),
        };
        first.children.push(CallNode {
            index: 3,
            depth: 3,
            call_type: CallType::Call,
            from: addr(60),
            to: Some(addr(50)),
            value: U256::zero(),
            gas: U256::zero(),
            gas_used: U256::zero(),
            input: Vec::new(),
            output: Vec::new(),
            error: None,
            children: Vec::new(),
        });

        let second = CallNode {
            index: 4,
            depth: 2,
            call_type: CallType::Call,
            from: addr(50),
            to: Some(addr(60)),
            value: U256::zero(),
            gas: U256::zero(),
            gas_used: U256::zero(),
            input: Vec::new(),
            output: Vec::new(),
            error: None,
            children: Vec::new(),
        };

        child.children = vec![first, second];
    }

    let detector = ReentrancyDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "reentrancy");
}

#[tokio::test]
async fn test_price_manipulation_detection_internal() {
    let mut analysis = nested_analysis();
    let token = addr(70);
    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(71), to: addr(72), amount: U256::from(3_000_000u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(72), to: addr(73), amount: U256::from(100u64), token_id: None, call_index: 2 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(73), to: addr(71), amount: U256::from(150u64), token_id: None, call_index: 3 },
    ];

    let detector = PriceManipulationDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "price_manipulation");
}

#[tokio::test]
async fn test_suspicious_liquidation_detection_internal() {
    let mut analysis = nested_analysis();
    let token = addr(80);
    analysis.token_transfers = vec![
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(81), to: addr(90), amount: U256::from(300_000u64), token_id: None, call_index: 1 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(82), to: addr(90), amount: U256::from(60u64), token_id: None, call_index: 2 },
        TokenTransfer { token_type: TokenType::Erc20, token_address: token, from: addr(90), to: addr(81), amount: U256::from(200u64), token_id: None, call_index: 3 },
    ];

    let detector = SuspiciousLiquidationDetector::new();
    let events = detector.detect_events(&analysis).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "suspicious_liquidation");
}
