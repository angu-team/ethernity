use crate::trace::{CallTrace, CallTree, CallNode, CallType};
use crate::utils;
use crate::TraceAnalysisConfig;
use ethereum_types::U256;

pub(super) struct TempNode {
    pub children: Vec<usize>,
}

pub fn build_call_tree(trace: &CallTrace, config: &TraceAnalysisConfig) -> Result<CallTree, ()> {
    let mut nodes = Vec::new();
    build_call_tree_recursive(trace, 0, &mut nodes, config)?;
    Ok(CallTree {
        root: CallNode {
            index: 0,
            depth: 0,
            call_type: trace.call_type.as_deref().map(CallType::from).unwrap_or(CallType::Call),
            from: utils::parse_address(&trace.from),
            to: if trace.to.is_empty() { None } else { Some(utils::parse_address(&trace.to)) },
            value: U256::from_dec_str(&trace.value).unwrap_or(U256::zero()),
            gas: U256::from_dec_str(&trace.gas).unwrap_or(U256::zero()),
            gas_used: U256::from_dec_str(&trace.gas_used).unwrap_or(U256::zero()),
            input: utils::decode_hex(&trace.input),
            output: utils::decode_hex(&trace.output),
            error: trace.error.clone(),
            children: Vec::new(),
        },
    })
}

fn build_call_tree_recursive(trace: &CallTrace, depth: usize, nodes: &mut Vec<TempNode>, config: &TraceAnalysisConfig) -> Result<(), ()> {
    if depth > config.max_depth {
        return Ok(());
    }

    let node = TempNode {
        children: Vec::new(),
    };
    let node_index = nodes.len();
    nodes.push(node);

    if let Some(calls) = &trace.calls {
        for child_call in calls {
            let child_index = nodes.len();
            build_call_tree_recursive(child_call, depth + 1, nodes, config)?;
            if let Some(parent) = nodes.get_mut(node_index) {
                parent.children.push(child_index);
            }
        }
    }

    Ok(())
}
