use std::env;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use chrono::Local;
use dashmap::DashMap;
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use ethers::prelude::*;
use futures::StreamExt;
use sandwich_victim::core::analyze_transaction;
use sandwich_victim::types::{Metrics, TransactionData};

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

    tokio::try_join!(
        mempool_listener(provider.clone(), rpc_client.clone(), ws_url.clone(), victims.clone()),
        block_listener(provider.clone(), victims.clone()),
        cleanup_task(victims.clone(), Duration::from_secs(600)),
    )?;

    Ok(())
}

async fn mempool_listener(
    provider: Arc<Provider<Ws>>,
    rpc_client: Arc<EthernityRpcClient>,
    ws_url: String,
    victims: Arc<DashMap<H256, VictimInfo>>,
) -> Result<()> {
    let mut stream = provider.subscribe_pending_txs().await?.transactions_unordered(10);
    println!("Escutando transações pendentes...");

    while let Some(res) = stream.next().await {
        
        let tx = match res {
            Ok(tx) => tx,
            Err(err) => {
                // println!("{:?}",err);
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
        
        if let Ok(result) = analyze_transaction(rpc_client.clone(), ws_url.clone(), tx_data, None).await {
            println!("{:?}",result);
            if result.potential_victim {
                let detected_at = Local::now();
                victims.insert(
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
    Ok(())
}

async fn block_listener(
    provider: Arc<Provider<Ws>>,
    victims: Arc<DashMap<H256, VictimInfo>>,
) -> Result<()> {
    let mut blocks = provider.subscribe_blocks().await?;
    println!("Escutando novos blocos...");

    while let Some(block) = blocks.next().await {
        let Some(number) = block.number else { continue };
        for hash in block.transactions {
            if let Some((_, info)) = victims.remove(&hash) {
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
    Ok(())
}

async fn cleanup_task(
    victims: Arc<DashMap<H256, VictimInfo>>,
    ttl: Duration,
) -> Result<()> {
    let mut interval = tokio::time::interval(ttl);
    loop {
        interval.tick().await;
        let now = Local::now();
        let max_age = chrono::Duration::from_std(ttl).unwrap();
        victims.retain(|_, info| now - info.detected_at <= max_age);
    }
}
