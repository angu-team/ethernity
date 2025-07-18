//! Analisa uma transação identificada por um hash utilizando um endpoint RPC.
//!
//! Uso:
//!
//! ```bash
//! cargo run -p sandwich-victim --example analyze_tx -- <RPC_ENDPOINT> <TX_HASH>
//! ```

use std::env;
use std::time::Duration;

use sandwich_victim::core::analyze_transaction;
use sandwich_victim::types::TransactionData;
use ethers::prelude::*;
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use std::sync::Arc;
use anyhow::anyhow;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Uso: {} <RPC_ENDPOINT> <TX_HASH>", args[0]);
        eprintln!("Exemplo: {} https://mainnet.infura.io/v3/YOURKEY 0x...", args[0]);
        std::process::exit(1);
    }

    let rpc = args[1].clone();
    let tx_hash: H256 = args[2].parse()?;

    let provider = Provider::<Http>::try_from(rpc.clone())?.interval(Duration::from_millis(100));
    let fetched = provider
        .get_transaction(tx_hash)
        .await?
        .ok_or_else(|| anyhow!("transação não encontrada"))?;

    let to = fetched.to.ok_or_else(|| anyhow!("transação sem destinatário"))?;

    let tx = TransactionData {
        from: fetched.from,
        to,
        data: fetched.input.to_vec(),
        value: fetched.value,
        gas: fetched.gas.as_u64(),
        gas_price: fetched.gas_price.unwrap_or_default(),
        nonce: fetched.nonce,
    };

    let rpc_client = Arc::new(EthernityRpcClient::new(RpcConfig {
        endpoint: rpc.clone(),
        ..Default::default()
    })
    .await?);

    let result = analyze_transaction(
        rpc_client,
        rpc,
        tx,
        fetched.block_number.map(|b| b.as_u64() - 1),
    )
    .await?;

    println!("Potencial vítima: {}", result.potential_victim);
    println!("Economicamente viável: {}", result.economically_viable);
    println!("Slippage: {:.4}", result.metrics.slippage);
    println!("Router: {:#x}", result.metrics.router_address);
    println!("Rota de tokens: {:?}", result.metrics.token_route);
    if let Some(hash) = result.simulated_tx {
        println!("Tx simulada: {hash:?}");
    }

    Ok(())
}
