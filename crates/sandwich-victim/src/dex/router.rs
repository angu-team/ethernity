use anyhow::Result;
use ethereum_types::{Address, U256, H256};
use ethers::abi::{AbiParser, Token};
use ethers::types::Log;
use ethers::utils::keccak256;
use ethernity_core::traits::RpcProvider;
use anyhow::anyhow;

/// Informações sobre o router detectado
#[derive(Debug, Clone)]
pub struct RouterInfo {
    pub address: Address,
    pub name: Option<String>,
    pub factory: Option<Address>,
}

/// Identifica dinamicamente o router utilizado na transação
pub async fn identify_router<P>(provider: &P, addr: Address) -> Result<RouterInfo>
where
    P: RpcProvider + Sync,
{
    // identificação genérica sem dependência de constantes "chumbadas"
    let name = None;

    // tenta obter a factory para confirmar ser um router
    let factory_abi = AbiParser::default()
        .parse_function("factory() view returns (address)")?;
    let call_res = provider
        .call(addr, factory_abi.encode_input(&[])? .into())
        .await;
    let factory = match call_res {
        Ok(out) => {
            let tokens = factory_abi.decode_output(&out)?;
            Some(tokens[0].clone().into_address().unwrap())
        }
        Err(_) => None,
    };

    // verifica se responde a getAmountsOut
    let amounts_out_abi = AbiParser::default()
        .parse_function("getAmountsOut(uint256,address[])")?;
    let test_data = amounts_out_abi.encode_input(&[
        Token::Uint(U256::one()),
        Token::Array(vec![Token::Address(addr), Token::Address(addr)]),
    ])?;
    let _ = provider
        .call(addr, test_data.into())
        .await
        .map_err(|e| anyhow!(e))
        .ok();

    Ok(RouterInfo {
        address: addr,
        name,
        factory,
    })
}

/// Tenta extrair o endereço do router a partir dos logs de simulação
pub fn router_from_logs(logs: &[Log]) -> Option<Address> {
    let swap_sig = H256::from_slice(
        keccak256("Swap(address,uint256,uint256,uint256,uint256,address)").as_slice(),
    );
    for log in logs {
        if log.topics.get(0) == Some(&swap_sig) && log.topics.len() > 1 {
            return Some(Address::from_slice(&log.topics[1].as_bytes()[12..]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use ethernity_core::error::{Result as CoreResult, Error};
    use ethernity_core::traits::RpcProvider;
    use ethernity_core::types::TransactionHash;

    struct DummyProvider {
        factory: Option<Address>,
    }

    #[async_trait]
    impl RpcProvider for DummyProvider {
        async fn get_transaction_trace(&self, _tx_hash: TransactionHash) -> CoreResult<Vec<u8>> {
            Ok(vec![])
        }

        async fn get_transaction_receipt(&self, _tx_hash: TransactionHash) -> CoreResult<Vec<u8>> {
            Ok(vec![])
        }

        async fn get_code(&self, _address: Address) -> CoreResult<Vec<u8>> {
            Ok(vec![])
        }

        async fn call(&self, _to: Address, data: Vec<u8>) -> CoreResult<Vec<u8>> {
            // factory() encoded call has only 4 bytes
            if data.len() == 4 {
                if let Some(addr) = self.factory {
                    let mut out = vec![0u8; 32];
                    out[12..].copy_from_slice(addr.as_bytes());
                    Ok(out)
                } else {
                    Err(Error::RpcError("not found".into()))
                }
            } else {
                Ok(vec![])
            }
        }

        async fn get_block_number(&self) -> CoreResult<u64> {
            Ok(0)
        }

        async fn get_block_hash(&self, _block_number: u64) -> CoreResult<H256> {
            Ok(H256::zero())
        }
    }

    #[test]
    fn extract_router_from_logs() {
        let router = Address::from_low_u64_be(42);
        let swap_sig = H256::from_slice(
            keccak256("Swap(address,uint256,uint256,uint256,uint256,address)").as_slice(),
        );
        let log = Log { topics: vec![swap_sig, router.into()], ..Default::default() };

        assert_eq!(router_from_logs(&[log]), Some(router));
    }

    #[test]
    fn router_from_logs_none() {
        let log = Log::default();
        assert_eq!(router_from_logs(&[log]), None);
    }

    #[tokio::test]
    async fn identify_router_returns_factory() {
        let factory = Address::from_low_u64_be(1);
        let provider = DummyProvider { factory: Some(factory) };
        let router = Address::from_low_u64_be(2);

        let info = identify_router(&provider, router).await.unwrap();
        assert_eq!(info.address, router);
        assert_eq!(info.factory, Some(factory));
    }

    #[tokio::test]
    async fn identify_router_no_factory() {
        let provider = DummyProvider { factory: None };
        let router = Address::from_low_u64_be(3);

        let info = identify_router(&provider, router).await.unwrap();
        assert_eq!(info.address, router);
        assert_eq!(info.factory, None);
    }

    #[test]
    fn router_from_logs_multiple_entries() {
        let other_log = Log::default();
        let router = Address::from_low_u64_be(55);
        let swap_sig = H256::from_slice(
            keccak256("Swap(address,uint256,uint256,uint256,uint256,address)").as_slice(),
        );
        let swap_log = Log { topics: vec![swap_sig, router.into()], ..Default::default() };

        assert_eq!(router_from_logs(&[other_log, swap_log]), Some(router));
    }
}
