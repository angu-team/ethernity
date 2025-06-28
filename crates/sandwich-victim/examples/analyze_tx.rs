//! Analisa uma transação fornecida em um arquivo JSON utilizando um endpoint RPC.
//!
//! É necessário compilar com a feature `anvil`:
//!
//! ```bash
//! cargo run -p sandwich-victim --example analyze_tx --features anvil -- <RPC_ENDPOINT> <ARQUIVO_JSON>
//! ```

#![cfg_attr(not(feature = "anvil"), allow(unused))]

#[cfg(not(feature = "anvil"))]
compile_error!("Este exemplo requer a feature 'anvil'. Utilize --features anvil");

use std::env;
use std::fs;

use sandwich_victim::analysis::analyze_transaction;
use sandwich_victim::types::TransactionData;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Uso: {} <RPC_ENDPOINT> <ARQUIVO_JSON>", args[0]);
        eprintln!("Exemplo: {} https://mainnet.infura.io/v3/YOURKEY tx.json", args[0]);
        std::process::exit(1);
    }

    let rpc = args[1].clone();
    let json = fs::read_to_string(&args[2])?;
    let tx: TransactionData = serde_json::from_str(&json)?;

    let result = analyze_transaction(rpc, tx).await?;

    println!("Potencial vítima: {}", result.potential_victim);
    println!("Economicamente viável: {}", result.economically_viable);
    println!("Slippage: {:.4}", result.metrics.slippage);
    println!("Router: {:?}", result.metrics.router_name);
    println!("Rota de tokens: {:?}", result.metrics.token_route);
    if let Some(hash) = result.simulated_tx {
        println!("Tx simulada: {hash:?}");
    }

    Ok(())
}
