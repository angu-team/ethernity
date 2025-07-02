use anyhow::Result;
use ethereum_types::{Address, U256};
use ethers::abi::{AbiParser, Token};
use ethernity_core::traits::RpcProvider;
use anyhow::anyhow;

/// Consulta as reservas de um par Uniswap V2-like
pub async fn get_pair_reserves<P>(provider: &P, pair: Address) -> Result<(U256, U256)>
where
    P: RpcProvider + Sync + ?Sized,
{
    let abi = AbiParser::default()
        .parse_function("getReserves() returns (uint112,uint112,uint32)")?;
    let data = abi.encode_input(&[])?;
    let out = provider.call(pair, data.into()).await.map_err(|e| anyhow!(e))?;
    let tokens = abi.decode_output(&out)?;
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
    P: RpcProvider + Sync + ?Sized,
{
    let abi = AbiParser::default()
        .parse_function("getPair(address,address) view returns (address)")?;
    let data = abi.encode_input(&[Token::Address(token_a), Token::Address(token_b)])?;
    let out = provider
        .call(factory, data.into())
        .await
        .map_err(|e| anyhow!(e))?;
    let tokens = abi.decode_output(&out)?;
    Ok(tokens[0].clone().into_address().unwrap())
}

/// Obtém os tokens de um par Uniswap V2-like
pub async fn get_pair_tokens<P>(provider: &P, pair: Address) -> Result<(Address, Address)>
where
    P: RpcProvider + Sync + ?Sized,
{
    let abi_token0 = AbiParser::default().parse_function("token0() view returns (address)")?;
    let data0 = abi_token0.encode_input(&[])?;
    let out0 = provider.call(pair, data0.into()).await.map_err(|e| anyhow!(e))?;
    let token0 = abi_token0
        .decode_output(&out0)?
        .get(0)
        .and_then(|t| t.clone().into_address())
        .ok_or_else(|| anyhow!("token0 decode failed"))?;

    let abi_token1 = AbiParser::default().parse_function("token1() view returns (address)")?;
    let data1 = abi_token1.encode_input(&[])?;
    let out1 = provider.call(pair, data1.into()).await.map_err(|e| anyhow!(e))?;
    let token1 = abi_token1
        .decode_output(&out1)?
        .get(0)
        .and_then(|t| t.clone().into_address())
        .ok_or_else(|| anyhow!("token1 decode failed"))?;

    Ok((token0, token1))
}
