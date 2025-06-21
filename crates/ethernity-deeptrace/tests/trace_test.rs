use ethernity_deeptrace::{CallTrace, CallTree, CallType};
use ethereum_types::Address;

fn mktrace(
    from: &str,
    to: &str,
    value: &str,
    gas: &str,
    gas_used: &str,
    input: &str,
    output: &str,
    call_type: Option<&str>,
    error: Option<&str>,
    calls: Option<Vec<CallTrace>>, 
) -> CallTrace {
    CallTrace {
        from: from.into(),
        to: to.into(),
        value: value.into(),
        gas: gas.into(),
        gas_used: gas_used.into(),
        input: input.into(),
        output: output.into(),
        call_type: call_type.map(|s| s.into()),
        error: error.map(|s| s.into()),
        calls,
    }
}

fn simple_trace() -> CallTrace {
    mktrace(
        "0x0000000000000000000000000000000000000001",
        "0x0000000000000000000000000000000000000002",
        "0",
        "21000",
        "21000",
        "0x",
        "0x",
        Some("CALL"),
        None,
        None,
    )
}

fn complex_trace() -> CallTrace {
    let child = mktrace(
        "0x0000000000000000000000000000000000000002",
        "0x0000000000000000000000000000000000000003",
        "5",
        "30000",
        "21000",
        "",
        "",
        Some("STATICCALL"),
        Some("revert"),
        None,
    );
    let grandchild = mktrace(
        "0x0000000000000000000000000000000000000003",
        "",
        "1",
        "1000",
        "500",
        "",
        "",
        Some("DELEGATECALL"),
        Some("fail"),
        None,
    );
    let mut child_with_grand = child.clone();
    child_with_grand.calls = Some(vec![grandchild]);
    mktrace(
        "0x0000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000001",
        "10",
        "50000",
        "30000",
        "",
        "",
        Some("CALL"),
        None,
        Some(vec![child_with_grand, child]),
    )
}

#[test]
fn test_calltype_from() {
    assert_eq!(CallType::from("CALL"), CallType::Call);
    assert_eq!(CallType::from("STATICCALL"), CallType::StaticCall);
    assert_eq!(CallType::from("DELEGATECALL"), CallType::DelegateCall);
    assert_eq!(CallType::from("CALLCODE"), CallType::CallCode);
    assert_eq!(CallType::from("CREATE"), CallType::Create);
    assert_eq!(CallType::from("CREATE2"), CallType::Create2);
    assert_eq!(CallType::from("SELFDESTRUCT"), CallType::SelfDestruct);
    assert_eq!(CallType::from("WHATEVER"), CallType::Unknown);
}

#[test]
fn test_simple_tree_functions() {
    let trace = simple_trace();
    let tree = CallTree::from_trace(&trace).unwrap();
    assert_eq!(tree.total_calls(), 1);
    assert_eq!(tree.max_depth(), 0);
    assert_eq!(tree.find_by_index(0).unwrap().from, Address::from_low_u64_be(1));
    assert!(tree.find_by_index(1).is_none());
    assert_eq!(tree.nodes_at_depth(0).len(), 1);
    assert!(tree.nodes_at_depth(1).is_empty());
    assert!(tree.failed_calls().is_empty());
    assert_eq!(tree.calls_to_address(&Address::from_low_u64_be(2)).len(), 1);
    assert_eq!(tree.calls_from_address(&Address::from_low_u64_be(1)).len(), 1);
    assert_eq!(tree.path_to_node(0), Some(vec![0]));
    let collected: Vec<usize> = tree.filter_nodes(|_| true).into_iter().map(|n| n.index).collect();
    assert_eq!(collected, vec![0]);
}

#[test]
fn test_traversal_and_paths() {
    let trace = complex_trace();
    let tree = CallTree::from_trace(&trace).unwrap();
    assert_eq!(tree.total_calls(), 4);
    assert_eq!(tree.max_depth(), 2);
    let mut preorder = Vec::new();
    tree.traverse_preorder(|n| preorder.push(n.index));
    assert_eq!(preorder, vec![0,1,2,3]);
    let mut postorder = Vec::new();
    tree.traverse_postorder(|n| postorder.push(n.index));
    assert_eq!(postorder, vec![2,1,3,0]);
    assert_eq!(tree.path_to_node(3), Some(vec![0,3]));
    assert!(tree.path_to_node(99).is_none());
    assert_eq!(tree.failed_calls().len(), 3);
    assert_eq!(tree.nodes_at_depth(1).len(), 2);
    assert_eq!(tree.calls_to_address(&Address::from_low_u64_be(3)).len(), 2);
    assert_eq!(tree.calls_from_address(&Address::from_low_u64_be(2)).len(), 2);
}

#[test]
#[should_panic]
fn test_invalid_from_panic() {
    let bad = mktrace("xyz", "", "0", "0", "0", "", "", None, None, None);
    CallTree::from_trace(&bad).unwrap();
}

#[test]
#[should_panic]
fn test_invalid_to_panic() {
    let bad = mktrace("0x1", "invalid", "0", "0", "0", "", "", None, None, None);
    CallTree::from_trace(&bad).unwrap();
}

#[test]
#[should_panic]
fn test_invalid_value_panic() {
    let bad = mktrace("0x1", "", "notnumber", "0", "0", "", "", None, None, None);
    CallTree::from_trace(&bad).unwrap();
}

#[test]
#[should_panic]
fn test_invalid_gas_panic() {
    let bad = mktrace("0x1", "", "0", "not", "0", "", "", None, None, None);
    CallTree::from_trace(&bad).unwrap();
}

#[test]
#[should_panic]
fn test_invalid_gas_used_panic() {
    let bad = mktrace("0x1", "", "0", "0", "bad", "", "", None, None, None);
    CallTree::from_trace(&bad).unwrap();
}

#[test]
#[should_panic]
fn test_invalid_input_panic() {
    let bad = mktrace("0x1", "", "0", "0", "0", "0xz", "", None, None, None);
    CallTree::from_trace(&bad).unwrap();
}

#[test]
#[should_panic]
fn test_invalid_output_panic() {
    let bad = mktrace("0x1", "", "0", "0", "0", "", "0xz", None, None, None);
    CallTree::from_trace(&bad).unwrap();
}

