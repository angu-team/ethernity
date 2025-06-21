use crate::{ContractCreation, ContractType};
use crate::trace::{CallTrace, CallType};
use crate::utils;
use ethereum_types::Address;
use std::collections::VecDeque;
use std::sync::Arc;

pub async fn extract_contract_creations(rpc: Arc<dyn ethernity_core::traits::RpcProvider>, trace: &CallTrace) -> Result<Vec<ContractCreation>, ()> {
    let mut creations = Vec::new();
    let mut queue = VecDeque::new();
    queue.push_back((trace, 0usize));
    while let Some((node, index)) = queue.pop_front() {
        let call_type = node.call_type.as_deref().map(CallType::from).unwrap_or(CallType::Call);
        if call_type == CallType::Create || call_type == CallType::Create2 {
            let contract_address = utils::parse_address(&node.to);
            if contract_address != Address::zero() {
                let bytecode = rpc.get_code(contract_address).await.map_err(|_| ())?;
                let contract_type = determine_contract_type(&bytecode)?;
                let from = utils::parse_address(&node.from);
                creations.push(ContractCreation {
                    creator: from,
                    contract_address,
                    init_code: utils::decode_hex(&node.input),
                    contract_type,
                    call_index: index,
                });
            }
        }
        if let Some(calls) = &node.calls {
            for (i, child) in calls.iter().enumerate() {
                queue.push_back((child, index + i + 1));
            }
        }
    }
    Ok(creations)
}

fn determine_contract_type(bytecode: &[u8]) -> Result<ContractType, ()> {
    let erc20_signatures: &[[u8; 4]] = &[
        [0x70, 0xa0, 0x82, 0x31],
        [0xa9, 0x05, 0x9c, 0xbb],
        [0x18, 0x16, 0x0d, 0xdd],
    ];
    let erc721_signatures: &[[u8; 4]] = &[
        [0x6f, 0xdd, 0x43, 0xe1],
        [0x6e, 0xb6, 0x1d, 0x3e],
        [0x42, 0x84, 0x2e, 0x0e],
    ];
    let selectors = crate::utils::BytecodeAnalyzer::extract_function_selectors(bytecode);
    let erc20_count = erc20_signatures.iter().filter(|sig| selectors.contains(sig)).count();
    if erc20_count >= 2 { return Ok(ContractType::Erc20Token); }
    let erc721_count = erc721_signatures.iter().filter(|sig| selectors.contains(sig)).count();
    if erc721_count >= 2 { return Ok(ContractType::Erc721Token); }
    let proxy_patterns = [
        &[0x36, 0x3d, 0x3d, 0x37],
        &[0x5c, 0x60, 0x20, 0x60],
    ];
    for pattern in &proxy_patterns {
        if crate::utils::BytecodeAnalyzer::contains_pattern(bytecode, *pattern) {
            return Ok(ContractType::Proxy);
        }
    }
    let create_ops = crate::utils::BytecodeAnalyzer::count_opcode(bytecode, 0xf0)
        + crate::utils::BytecodeAnalyzer::count_opcode(bytecode, 0xf5);
    if create_ops > 1 { return Ok(ContractType::Factory); }
    Ok(ContractType::Unknown)
}
