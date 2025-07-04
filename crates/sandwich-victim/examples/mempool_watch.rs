use std::env;
use std::time::Duration;

use anyhow::Result;
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use ethers::prelude::*;
use futures::StreamExt;
use sandwich_victim::core::analyze_transaction;
use sandwich_victim::types::{Metrics, TransactionData};
use std::sync::Arc;
use chrono::Local;
use dashmap::DashMap;

#[derive(Debug, Clone)]
struct VictimInfo {
    detected_at: chrono::DateTime<chrono::Local>,
    metrics: Metrics,
}

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

    let victims: Arc<DashMap<H256, VictimInfo>> = Arc::new(DashMap::new());

    // ---------- Mempool listener ----------
    let mempool_provider = provider.clone();
    let mempool_client = rpc_client.clone();
    let mempool_victims = victims.clone();
    let mempool_ws = ws_url.clone();
    tokio::spawn(async move {
        let mut stream = mempool_provider
            .subscribe_pending_txs()
            .await
            .expect("failed to subscribe pending txs")
            .transactions_unordered(10);
        println!("Escutando transações pendentes...");

        while let Some(res) = stream.next().await {
            let tx = match res {
                Ok(tx) => tx,
                Err(_) => continue,
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

            if let Ok(result) = analyze_transaction(mempool_client.clone(), mempool_ws.clone(), tx_data, None).await {
                if result.potential_victim {
                    let detected_at = Local::now();
                    mempool_victims.insert(
                        tx.hash,
                        VictimInfo { detected_at, metrics: result.metrics },
                    );
                    println!(
                        "[mempool {}] possível vítima {:?}",
                        detected_at.format("%H:%M:%S%.3f"),
                        tx.hash
                    );
                }
            }
        }
    });

    // ---------- Block listener ----------
    let block_provider = provider.clone();
    let block_victims = victims.clone();
    tokio::spawn(async move {
        let mut blocks = block_provider
            .subscribe_blocks()
            .await
            .expect("failed to subscribe blocks");
        println!("Escutando novos blocos...");

        while let Some(block) = blocks.next().await {
            let Some(number) = block.number else { continue };
            for hash in block.transactions {
                if let Some((_, info)) = block_victims.remove(&hash) {
                    println!("Tx {:?} confirmada no bloco {}", hash, number);
                    println!(
                        "Detectada em {}",
                        info.detected_at.format("%H:%M:%S%.3f")
                    );
                    println!("Slippage: {:.4}", info.metrics.slippage);
                    println!("Router: {:?}", info.metrics.router_name);
                    println!("Rota de tokens: {:?}", info.metrics.token_route);
                }
            }
        }
    });

    // keep running
    futures::future::pending::<()>().await;

    Ok(())
}
