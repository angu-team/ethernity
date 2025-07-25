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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use ethernity_core::error::Error;

    struct MockRpc { code: Vec<u8> }

    #[async_trait]
    impl ethernity_core::traits::RpcProvider for MockRpc {
        async fn get_transaction_trace(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_transaction_receipt(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_code(&self, _address: Address) -> ethernity_core::error::Result<Vec<u8>> { Ok(self.code.clone()) }
        async fn call(&self, _to: Address, _data: Vec<u8>) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_block_number(&self) -> ethernity_core::error::Result<u64> { Ok(0) }
        async fn get_block_hash(&self, _block_number: u64) -> ethernity_core::error::Result<ethereum_types::H256> { Ok(ethereum_types::H256::zero()) }
    }

    struct CountingRpc {
        code: Vec<u8>,
        calls: Mutex<Vec<Address>>, 
    }

    #[async_trait]
    impl ethernity_core::traits::RpcProvider for CountingRpc {
        async fn get_transaction_trace(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_transaction_receipt(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_code(&self, address: Address) -> ethernity_core::error::Result<Vec<u8>> {
            self.calls.lock().unwrap().push(address);
            Ok(self.code.clone())
        }
        async fn call(&self, _to: Address, _data: Vec<u8>) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_block_number(&self) -> ethernity_core::error::Result<u64> { Ok(0) }
        async fn get_block_hash(&self, _block_number: u64) -> ethernity_core::error::Result<ethereum_types::H256> { Ok(ethereum_types::H256::zero()) }
    }

    struct ErrorRpc;

    #[async_trait]
    impl ethernity_core::traits::RpcProvider for ErrorRpc {
        async fn get_transaction_trace(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_transaction_receipt(&self, _tx: ethernity_core::types::TransactionHash) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_code(&self, _address: Address) -> ethernity_core::error::Result<Vec<u8>> { Err(Error::Other("fail".into())) }
        async fn call(&self, _to: Address, _data: Vec<u8>) -> ethernity_core::error::Result<Vec<u8>> { Ok(vec![]) }
        async fn get_block_number(&self) -> ethernity_core::error::Result<u64> { Ok(0) }
        async fn get_block_hash(&self, _block_number: u64) -> ethernity_core::error::Result<ethereum_types::H256> { Ok(ethereum_types::H256::zero()) }
    }

    #[tokio::test]
    async fn test_extract_contract_creations() {
        let trace = CallTrace {
            from: "0x01".into(), gas: "0".into(), gas_used: "0".into(),
            to: "0x0000000000000000000000000000000000000100".into(), input: "0x".into(), output: "0x".into(), value: "0".into(), error: None,
            calls: None, call_type: Some("CREATE".into())
        };
        let rpc = Arc::new(MockRpc { code: vec![0x63,0x70,0xa0,0x82,0x31,0x00,0x00,0x63,0xa9,0x05,0x9c,0xbb,0x00,0x00,0x00] });
        let res = extract_contract_creations(rpc, &trace).await.unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].contract_type, ContractType::Erc20Token);
        assert_eq!(res[0].call_index, 0);
    }

    #[tokio::test]
    async fn test_extract_contract_creations_nested_and_indices() {
        let child = CallTrace {
            from: "0x02".into(), gas: "0".into(), gas_used: "0".into(),
            to: "0x0000000000000000000000000000000000000200".into(), input: "0x".into(), output: "0x".into(), value: "0".into(),
            error: None, calls: None, call_type: Some("CREATE2".into())
        };
        let root = CallTrace {
            from: "0x01".into(), gas: "0".into(), gas_used: "0".into(),
            to: "0x0000000000000000000000000000000000000100".into(), input: "0x".into(), output: "0x".into(), value: "0".into(),
            error: None, calls: Some(vec![child]), call_type: Some("CREATE".into())
        };
        let rpc = Arc::new(CountingRpc { code: vec![0x36,0x3d,0x3d,0x37], calls: Mutex::new(Vec::new()) });
        let res = extract_contract_creations(rpc.clone(), &root).await.unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].call_index, 0);
        assert_eq!(res[1].call_index, 1);
        let calls = rpc.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
    }

    #[tokio::test]
    async fn test_extract_contract_creations_non_create_and_zero() {
        let trace = CallTrace {
            from: "0x01".into(), gas: "0".into(), gas_used: "0".into(),
            to: "0x".into(), input: "0x".into(), output: "0x".into(), value: "0".into(),
            error: None, calls: None, call_type: Some("CALL".into())
        };
        let rpc = Arc::new(CountingRpc { code: vec![], calls: Mutex::new(Vec::new()) });
        let res = extract_contract_creations(rpc.clone(), &trace).await.unwrap();
        assert!(res.is_empty());
        assert!(rpc.calls.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_extract_contract_creations_error_propagation() {
        let trace = CallTrace {
            from: "0x01".into(), gas: "0".into(), gas_used: "0".into(),
            to: "0x0000000000000000000000000000000000000100".into(), input: "0x".into(), output: "0x".into(), value: "0".into(),
            error: None, calls: None, call_type: Some("CREATE".into())
        };
        let rpc = Arc::new(ErrorRpc);
        assert!(extract_contract_creations(rpc, &trace).await.is_err());
    }

    #[tokio::test]
    async fn test_extract_contract_creations_zero_address_skips_rpc() {
        let trace = CallTrace {
            from: "0x01".into(), gas: "0".into(), gas_used: "0".into(),
            to: "0x0000000000000000000000000000000000000000".into(), input: "0x".into(), output: "0x".into(), value: "0".into(),
            error: None, calls: None, call_type: Some("CREATE".into())
        };
        let rpc = Arc::new(CountingRpc { code: vec![], calls: Mutex::new(Vec::new()) });
        let res = extract_contract_creations(rpc.clone(), &trace).await.unwrap();
        assert!(res.is_empty());
        assert!(rpc.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn test_determine_contract_type_all_paths() {
        // ERC20
        let code = vec![0x63,0x70,0xa0,0x82,0x31,0x00,0x00,0x63,0xa9,0x05,0x9c,0xbb,0x00,0x00];
        assert_eq!(determine_contract_type(&code).unwrap(), ContractType::Erc20Token);
        // ERC721
        let code = vec![0x63,0x6f,0xdd,0x43,0xe1,0x00,0x00,0x63,0x6e,0xb6,0x1d,0x3e,0x00,0x00];
        assert_eq!(determine_contract_type(&code).unwrap(), ContractType::Erc721Token);
        // Proxy
        let code = vec![0x36,0x3d,0x3d,0x37];
        assert_eq!(determine_contract_type(&code).unwrap(), ContractType::Proxy);
        // Factory
        let code = vec![0xf0,0xf5,0xf0];
        assert_eq!(determine_contract_type(&code).unwrap(), ContractType::Factory);
        // Unknown
        let code = vec![0u8];
        assert_eq!(determine_contract_type(&code).unwrap(), ContractType::Unknown);
    }
}
