use anyhow::Result;
use ethereum_types::{Address, U256};
use ethers::abi::{AbiParser, Token};
use ethers::prelude::*;

/// Consulta as reservas de um par Uniswap V2-like
pub async fn get_pair_reserves<P>(provider: &P, pair: Address) -> Result<(U256, U256)>
where
    P: Middleware,
    <P as Middleware>::Error: 'static,
{
    let abi = AbiParser::default()
        .parse_function("getReserves() returns (uint112,uint112,uint32)")?;
    let tx = ethers::types::TransactionRequest {
        to: Some(NameOrAddress::Address(pair)),
        data: Some(abi.encode_input(&[])? .into()),
        ..Default::default()
    };
    let out = provider.call(&tx.into(), None).await?;
    let tokens = abi.decode_output(&out.0)?;
    Ok((
        tokens[0].clone().into_uint().unwrap(),
        tokens[1].clone().into_uint().unwrap(),
    ))
}

/// Obtém o endereço do par para dois tokens via factory
pub async fn get_pair_address<P>(
    provider: &P,
    factory: Address,
    token_a: Address,
    token_b: Address,
) -> Result<Address>
where
    P: Middleware,
    <P as Middleware>::Error: 'static,
{
    let abi = AbiParser::default()
        .parse_function("getPair(address,address) view returns (address)")?;
    let data = abi.encode_input(&[Token::Address(token_a), Token::Address(token_b)])?;
    let req = ethers::types::TransactionRequest {
        to: Some(NameOrAddress::Address(factory)),
        data: Some(data.into()),
        ..Default::default()
    };
    let out = provider.call(&req.into(), None).await?;
    let tokens = abi.decode_output(&out.0)?;
    Ok(tokens[0].clone().into_address().unwrap())
}
