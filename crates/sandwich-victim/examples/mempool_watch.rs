use std::env;
use std::time::Duration;

use anyhow::Result;
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use ethers::prelude::*;
use futures::StreamExt;
use sandwich_victim::core::analyze_transaction;
use sandwich_victim::types::TransactionData;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <WS_RPC_ENDPOINT>", args[0]);
        eprintln!("Exemplo: {} ws://localhost:8546", args[0]);
        std::process::exit(1);
    }

    let ws_url = args[1].clone();
    let ws = Ws::connect(ws_url.clone()).await?;
    let provider = Provider::new(ws).interval(Duration::from_millis(1000));

    let rpc_client = Arc::new(
        EthernityRpcClient::new(RpcConfig {
            endpoint: ws_url.clone(),
            ..Default::default()
        })
        .await?,
    );

    let mut stream = provider
        .subscribe_pending_txs()
        .await?
        .transactions_unordered(10);
    println!("Escutando transações pendentes...");

    while let Some(res) = stream.next().await {
        let tx = match res {
            Ok(tx) => tx,
            Err(err) => {
                // eprintln!("Erro ao obter transação: {err}");
                continue;
            }
        };
        let Some(to) = tx.to else { continue };
        let tx_data = TransactionData {
            from: tx.from,
            to,
            data: tx.input.to_vec(),
            value: tx.value,
            gas: tx.gas.as_u64(),
            gas_price: tx.gas_price.unwrap_or_default(),
            nonce: tx.nonce,
        };

        match analyze_transaction(rpc_client.clone(), ws_url.clone(), tx_data, None).await {
            Ok(result) if result.potential_victim => {
                println!("Possível vítima: {:?}", tx.hash);
                println!("Slippage: {:.4}", result.metrics.slippage);
                println!("Router: {:?}", result.metrics.router_name);
                println!("Rota de tokens: {:?}", result.metrics.token_route);
            }
            Ok(_) => {}
            Err(err) => {
                // eprintln!("Falha ao analisar tx {:?}: {err}", tx.hash);
            }
        }
    }

    Ok(())
}
