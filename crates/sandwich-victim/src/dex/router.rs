use anyhow::Result;
use ethereum_types::{Address, U256};
use ethers::abi::{AbiParser, Token};
use ethers::prelude::*;

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
    P: Middleware,
    <P as Middleware>::Error: 'static,
{
    const UNISWAP_V2_BYTES: [u8; 20] = [
        0x7a, 0x25, 0x0d, 0x56, 0x30, 0xb4, 0xcf, 0x53,
        0x97, 0x39, 0xdf, 0x2c, 0x5d, 0xac, 0xb4, 0xc6,
        0x59, 0xf2, 0x48, 0x8d,
    ];
    const SUSHI_V2_BYTES: [u8; 20] = [
        0xd9, 0xe1, 0xce, 0x17, 0xf2, 0x64, 0x1f, 0x24,
        0xae, 0x83, 0x63, 0x7a, 0xb6, 0x6a, 0x2c, 0xca,
        0x9c, 0x37, 0x8b, 0x9f,
    ];

    let name = if addr == Address::from(UNISWAP_V2_BYTES) {
        Some("UniswapV2".to_string())
    } else if addr == Address::from(SUSHI_V2_BYTES) {
        Some("SushiSwap".to_string())
    } else {
        None
    };

    // tenta obter a factory para confirmar ser um router
    let factory_abi = AbiParser::default()
        .parse_function("factory() view returns (address)")?;
    let req = ethers::types::TransactionRequest {
        to: Some(NameOrAddress::Address(addr)),
        data: Some(factory_abi.encode_input(&[])? .into()),
        ..Default::default()
    };
    let call_res = provider.call(&req.into(), None).await;
    let factory = match call_res {
        Ok(out) => {
            let tokens = factory_abi.decode_output(&out.0)?;
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
    let req = ethers::types::TransactionRequest {
        to: Some(NameOrAddress::Address(addr)),
        data: Some(test_data.into()),
        ..Default::default()
    };
    let _ = provider.call(&req.into(), None).await.ok();

    Ok(RouterInfo {
        address: addr,
        name,
        factory,
    })
}
