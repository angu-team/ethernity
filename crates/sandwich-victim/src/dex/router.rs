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
