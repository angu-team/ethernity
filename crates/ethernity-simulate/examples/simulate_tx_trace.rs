use std::env;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use ethers::prelude::*;
use ethers::providers::JsonRpcClient;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::utils::Anvil;
use tracing::info;

fn to_typed(tx: &Transaction) -> TypedTransaction {
    match tx.transaction_type.map(|v| v.as_u64()) {
        Some(2) => {
            let req = Eip1559TransactionRequest {
                from: Some(tx.from),
                to: tx.to.map(NameOrAddress::Address),
                gas: Some(tx.gas),
                value: Some(tx.value),
                data: Some(tx.input.clone()),
                nonce: Some(tx.nonce),
                access_list: tx.access_list.clone().unwrap_or_default(),
                max_priority_fee_per_gas: tx.max_priority_fee_per_gas,
                max_fee_per_gas: tx.max_fee_per_gas,
                chain_id: tx.chain_id.map(|c| U64::from(c.as_u64())),
            };
            req.into()
        }
        Some(1) => {
            let req = TransactionRequest {
                from: Some(tx.from),
                to: tx.to.map(NameOrAddress::Address),
                gas: Some(tx.gas),
                gas_price: tx.gas_price,
                value: Some(tx.value),
                data: Some(tx.input.clone()),
                nonce: Some(tx.nonce),
                chain_id: tx.chain_id.map(|c| U64::from(c.as_u64())),
            };
            req
                .with_access_list(tx.access_list.clone().unwrap_or_default())
                .into()
        }
        _ => {
            let req = TransactionRequest {
                from: Some(tx.from),
                to: tx.to.map(NameOrAddress::Address),
                gas: Some(tx.gas),
                gas_price: tx.gas_price,
                value: Some(tx.value),
                data: Some(tx.input.clone()),
                nonce: Some(tx.nonce),
                chain_id: tx.chain_id.map(|c| U64::from(c.as_u64())),
            };
            req.into()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Uso: {} <RPC_ENDPOINT> <TX_HASH>", args[0]);
        std::process::exit(1);
    }
    let rpc = &args[1];
    let tx_hash: H256 = args[2].parse().context("hash invalido")?;

    if rpc.starts_with("ws") {
        let ws = Ws::connect(rpc)
            .await
            .context("falha ao conectar via websocket")?;
        let provider = Provider::new(ws).interval(Duration::from_millis(1));
        simulate(provider, rpc, tx_hash).await
    } else {
        let http = Provider::<Http>::try_from(rpc)
            .context("falha ao conectar via http")?
            .interval(Duration::from_millis(1));
        simulate(http, rpc, tx_hash).await
    }
}

async fn simulate<P>(
    provider: Provider<P>,
    rpc: &str,
    tx_hash: H256,
) -> Result<()>
where
    P: JsonRpcClient + 'static,
{
    let tx = provider
        .get_transaction(tx_hash)
        .await
        .context("falha ao buscar transacao")?
        .context("transacao nao encontrada")?;

    let block = tx.block_number.context("transacao pendente")?;
    info!("Transacao localizada no bloco {}", block);

    provider
        .get_block(block)
        .await
        .context("falha ao obter bloco original")?
        .context("bloco nao encontrado")?;

    let start = Instant::now();

    let anvil = Anvil::new()
        .fork(rpc)
        .fork_block_number(block.as_u64())
        .args(&["--auto-impersonate".to_string()])
        .spawn();

    let anvil_provider = Provider::<Http>::try_from(anvil.endpoint())
        .map_err(|e| anyhow::anyhow!(e))?
        .interval(Duration::from_millis(1));

    let typed = to_typed(&tx);
    let pending = anvil_provider
        .send_transaction(typed, None)
        .await
        .context("falha ao enviar transacao")?;
    let receipt = pending
        .await
        .context("falha ao aguardar transacao")?
        .context("sem recibo")?;
    info!("Transacao simulada: {:?}", receipt.transaction_hash);

    let params = [
        serde_json::to_value(receipt.transaction_hash)?,
        serde_json::json!({"tracer": "callTracer", "timeout": "60s"}),
    ];
    let trace: serde_json::Value = anvil_provider
        .request("debug_traceTransaction", params)
        .await
        .context("falha ao obter trace")?;
    info!("Trace obtido: {}", trace);

    drop(anvil);
    let elapsed = start.elapsed();
    println!(
        "Tempo total: {}.{:03} segundos",
        elapsed.as_secs(),
        elapsed.subsec_millis()
    );

    Ok(())
}
