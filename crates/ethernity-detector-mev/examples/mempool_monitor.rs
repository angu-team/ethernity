use std::env;
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Duration;

use ethernity_detector_mev::*;
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use ethernity_core::{traits::RpcProvider, error::Result};
use ethernity_core::types::TransactionHash;
use ethers::providers::{Middleware, Provider, Ws, StreamExt};
use ethers::types::U256;
use ethereum_types::{Address, H256};

#[derive(Clone)]
struct SharedRpc(Arc<EthernityRpcClient>);

#[async_trait::async_trait]
impl RpcProvider for SharedRpc {
    async fn get_transaction_trace(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        self.0.get_transaction_trace(tx_hash).await
    }

    async fn get_transaction_receipt(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        self.0.get_transaction_receipt(tx_hash).await
    }

    async fn get_code(&self, address: Address) -> Result<Vec<u8>> {
        self.0.get_code(address).await
    }

    async fn call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>> {
        self.0.call(to, data).await
    }

    async fn get_block_number(&self) -> Result<u64> {
        self.0.get_block_number().await
    }

    async fn get_block_hash(&self, block_number: u64) -> Result<H256> {
        self.0.get_block_hash(block_number).await
    }
}

fn u256_to_f64(v: U256) -> f64 {
    v.low_u128() as f64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <ENDPOINT_WS>", args[0]);
        eprintln!("Exemplo: {} wss://mainnet.infura.io/ws/v3/YOURKEY", args[0]);
        std::process::exit(1);
    }

    // Endpoint WebSocket para leitura do mempool
    let endpoint = args[1].clone();

    // Cliente RPC utilizado para consultas de estado e blocos
    let rpc_cfg = RpcConfig { endpoint: endpoint.clone(), ..Default::default() };
    let rpc_client = Arc::new(EthernityRpcClient::new(rpc_cfg).await?);
    let rpc = SharedRpc(rpc_client.clone());

    // Conexão WebSocket usando ethers para subscrever novas transações pendentes
    let ws = Ws::connect(&endpoint).await?;
    let provider = Provider::new(ws);

    // Canal de eventos para o supervisor
    let (bus, rx_events) = EventBus::new(1024);
    let (tx_groups, mut rx_groups) = tokio::sync::mpsc::channel(32);

    // Supervisor da mempool responsável por agrupar e gerar janelas
    let supervisor = MempoolSupervisor::new(rpc.clone(), 2, Duration::from_secs(5), 50);
    tokio::spawn(supervisor.process_stream(rx_events, tx_groups));

    // Tarefa: monitorar transações pendentes
    let tx_sender = bus.sender();
    let tagger = TxNatureTagger::new(rpc.clone());
    tokio::spawn(async move {
        let mut sub = match provider.subscribe_pending_txs().await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Falha ao subscrever mempool: {e}");
                return;
            }
        };
        while let Some(hash) = sub.next().await {
            match provider.get_transaction(hash).await {
                Ok(Some(tx)) => {
                    if let Some(to) = tx.to {
                        let input = tx.input.0.clone();
                        match tagger.analyze(to, &input, tx.hash).await {
                            Ok(nature) => {
                                let annotated = AnnotatedTx {
                                    tx_hash: tx.hash,
                                    token_paths: nature.token_paths,
                                    targets: nature.targets,
                                    tags: nature.tags,
                                    first_seen: chrono::Utc::now().timestamp() as u64,
                                    gas_price: tx.gas_price.map(u256_to_f64).unwrap_or_default(),
                                    max_priority_fee_per_gas: tx.max_priority_fee_per_gas.map(u256_to_f64),
                                    confidence: nature.confidence,
                                };
                                let _ = tx_sender
                                    .send(SupervisorEvent::NewTxObserved(annotated))
                                    .await;
                            }
                            Err(e) => eprintln!("Tagger error: {e}"),
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => eprintln!("Erro ao obter transação: {e}"),
            }
        }
    });

    // Tarefa: acompanhar novos blocos para sincronização
    let tx_sender_block = bus.sender();
    tokio::spawn(async move {
        let mut last = 0u64;
        loop {
            match rpc_client.get_block_number().await {
                Ok(n) if n != last => {
                    last = n;
                    let _ = tx_sender_block.send(SupervisorEvent::BlockAdvanced(BlockMetadata { number: n })).await;
                }
                _ => {}
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // Avaliadores de impacto e detecção de ataques
    let repo_dir = std::env::temp_dir().join("mev_example_db");
    if repo_dir.exists() {
        // Remove banco de dados antigo para evitar erro de lock
        std::fs::remove_dir_all(&repo_dir)?;
    }
    let repo = StateSnapshotRepository::open(rpc.clone(), &repo_dir)?;
    let mut impact_eval = StateImpactEvaluator::default();
    let detector = AttackDetector::new(1.0, 10);

    println!("Monitorando mempool em {endpoint} ...");
    while let Some(gr) = rx_groups.recv().await {
        println!("\nGrupo {:x} detectado com {} transações", gr.group.group_key, gr.group.txs.len());
        let mut map = HashMap::new();
        map.insert(gr.group.group_key, gr.group.clone());
        repo.snapshot_groups(&map, gr.metadata.window_id, SnapshotProfile::Basic).await?;
        if let Some(target) = gr.group.targets.first() {
            if let Some(snap) = repo.get_state(*target, gr.metadata.window_id, SnapshotProfile::Basic) {
                let impact = impact_eval.evaluate_group(&gr.group, &[], &snap);
                if let Some(verdict) = detector.analyze_group(&gr.group) {
                    println!("  Possível ataque MEV com confiança {:.2}", verdict.confidence);
                    for at in verdict.attack_types {
                        println!("    - {:?}", at);
                    }
                } else {
                    println!("  Nenhum ataque evidente identificado");
                }
                println!("  Score de oportunidade: {:.2}", impact.opportunity_score);
            }
        }
    }

    Ok(())
}

