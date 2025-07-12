use std::env;
use std::time::Duration;

use anyhow::Context;
use ethers::prelude::*;
use ethers::utils::parse_ether;
use ethernity_simulate::{AnvilProvider, SimulationProvider, SimulationSession};
use tracing::info;

/// Endereço da primeira conta padrão do Anvil
const ACCOUNT_A: &str = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
/// Endereço da segunda conta padrão do Anvil
const ACCOUNT_B: &str = "0x70997970c51812dc3a010c7d01b50e0d17dc79c8";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <RPC_WS_ENDPOINT>", args[0]);
        std::process::exit(1);
    }
    let rpc = &args[1];

    // Conecta no endpoint informado apenas para obter o número do bloco atual
    let ws = Ws::connect(rpc).await.context("falha ao conectar via websocket")?;
    let provider_ws = Provider::new(ws);
    let block = provider_ws
        .get_block_number()
        .await
        .context("falha ao obter bloco atual")?;
    info!("Forkando no bloco {}", block);

    // Cria a sessão de simulação baseada no fork
    let sim_provider = AnvilProvider;
    let session = sim_provider
        .create_session(rpc, Some(block.as_u64()), Duration::from_secs(60))
        .await
        .context("falha ao criar sessão")?;
    let id = { session.lock().await.id };
    info!("Sessão {id} criada");

    // Monta uma transação simples entre contas desbloqueadas do Anvil
    let tx = TransactionRequest::pay(ACCOUNT_B.parse::<Address>()?, parse_ether(1u64)?)
        .from(ACCOUNT_A.parse::<Address>()?);

    // Envia a transação dentro da sessão
    let receipt = session
        .send_transaction(&tx.into())
        .await
        .context("falha ao enviar transação")?;
    info!("Transação enviada: {:?}", receipt.transaction_hash);

    // Finaliza a sessão
    session.close().await;
    info!("Sessão encerrada");

    Ok(())
}
