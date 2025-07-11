use std::env;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use ethers::prelude::*;
use futures::StreamExt;
use sandwich_victim::core::analyze_transaction;
use sandwich_victim::types::TransactionData;

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
    let provider = Arc::new(Provider::new(ws).interval(Duration::from_millis(1000)));

    let rpc_client = Arc::new(
        EthernityRpcClient::new(RpcConfig {
            endpoint: ws_url.clone(),
            ..Default::default()
        })
        .await?,
    );

    mempool_listener(provider.clone(), rpc_client.clone(), ws_url.clone()).await?;

    Ok(())
}

async fn mempool_listener(
    provider: Arc<Provider<Ws>>,
    rpc_client: Arc<EthernityRpcClient>,
    ws_url: String,
) -> Result<()> {
    let stream = provider.subscribe_pending_txs().await?.transactions_unordered(1);
    println!("Escutando transações pendentes...");

    stream
        .for_each_concurrent(usize::MAX, |res| {
            let rpc_client = rpc_client.clone();
            let ws_url = ws_url.clone();
            async move {
                let tx = match res {
                    Ok(tx) => tx,
                    Err(_) => return,
                };

                let Some(to) = tx.to else { return };
                let tx_data = TransactionData {
                    from: tx.from,
                    to,
                    data: tx.input.to_vec(),
                    value: tx.value,
                    gas: tx.gas.as_u64(),
                    gas_price: tx.gas_price.unwrap_or_default(),
                    nonce: tx.nonce,
                };

                match analyze_transaction(rpc_client, "http://148.251.183.245:8545".to_string(), tx_data, None).await {
                    Ok(result) if result.potential_victim => {
                        println!("possível vítima {:?}\n{:#?}", tx.hash, result.metrics);
                    }
                    Ok(_) => {}
                    Err(err) => eprintln!("Erro ao analisar tx {:?}: {err}", tx.hash),
                }
            }
        })
        .await;
    Ok(())
}
