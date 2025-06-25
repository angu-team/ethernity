use std::env;
use std::sync::Arc;

use ethernity_detector_mev::*;
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use web3::types::{Block, Transaction};

/// Converte U256 para f64 de forma segura.
fn u256_to_f64(value: web3::types::U256) -> f64 {
    // web3::types::U256 -> primitive u128 -> f64
    let val: u128 = value.into();
    val as f64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Uso: {} <RPC_ENDPOINT> [BLOCO]", args[0]);
        eprintln!("Exemplo: {} https://mainnet.infura.io/v3/YOURKEY 17", args[0]);
        std::process::exit(1);
    }

    let endpoint = args[1].clone();
    let block_number: Option<u64> = if args.len() >= 3 {
        Some(args[2].parse::<u64>()?)
    } else {
        None
    };

    // Configuração simples do cliente RPC
    let rpc_config = RpcConfig { endpoint: endpoint.clone(), ..Default::default() };
    let rpc = Arc::new(EthernityRpcClient::new(rpc_config).await?);

    // Determina o bloco a ser analisado (atual se não fornecido)
    let target_block = match block_number {
        Some(b) => b,
        None => rpc.get_block_number().await?,
    };

    println!("Analisando bloco {target_block}...");

    // Recupera o bloco e converte para estrutura do web3
    let block_bytes = rpc.get_block(target_block).await?;
    let block: Block<Transaction> = serde_json::from_slice(&block_bytes)?;

    // Instancia componentes principais
    let tagger = TxNatureTagger::new(rpc.clone());
    let mut aggregator = TxAggregator::new();

    // Itera pelas transações do bloco classificando cada uma
    for tx in block.transactions.iter() {
        if let (Some(to_addr), Some(input)) = (tx.to, Some(tx.input.0.clone())) {
            let tx_hash = tx.hash;
            let first_seen = block.timestamp.as_u64();
            let gas_price = tx.gas_price.map(u256_to_f64).unwrap_or_default();
            let max_priority = tx
                .max_priority_fee_per_gas
                .map(u256_to_f64);

            match tagger.analyze(to_addr, &input, tx_hash).await {
                Ok(nature) => {
                    let annotated = AnnotatedTx {
                        tx_hash,
                        token_paths: nature.token_paths,
                        targets: nature.targets,
                        tags: nature.tags,
                        first_seen,
                        gas_price,
                        max_priority_fee_per_gas: max_priority,
                        confidence: nature.confidence,
                    };
                    aggregator.add_tx(annotated);
                }
                Err(e) => {
                    eprintln!("Falha ao analisar transação {tx_hash:?}: {e}");
                }
            }
        }
    }

    if aggregator.groups().is_empty() {
        println!("Nenhum grupo relevante encontrado no bloco.");
        return Ok(());
    }

    // Repositório de snapshots utilizado para consultar estado on-chain
    let snapshot_dir = std::env::temp_dir().join("mev_detector_db");
    let repo = StateSnapshotRepository::open(rpc.clone(), &snapshot_dir)?;

    // Captura um snapshot básico de cada par envolvido
    repo.snapshot_groups(aggregator.groups(), target_block, SnapshotProfile::Basic)
        .await?;

    // Avaliadores e detector de ataques
    let mut impact_eval = StateImpactEvaluator::default();
    let attack_detector = AttackDetector::new(1.0, 10);

    for group in aggregator.groups().values() {
        println!("\nGrupo {:x} ({} txs)", group.group_key, group.txs.len());
        if let Some(target) = group.targets.first() {
            if let Some(snapshot) = repo.get_state(*target, target_block, SnapshotProfile::Basic) {
                // Avalia impacto econômico potencial
                let impact = impact_eval.evaluate_group(group, &[], &snapshot);
                println!("  Score de oportunidade: {:.2}", impact.opportunity_score);

                // Detecta padrões de ataque MEV
                if let Some(verdict) = attack_detector.analyze_group(group) {
                    println!("  Possível ataque MEV detectado com confiança {:.2}", verdict.confidence);
                    for at in verdict.attack_types {
                        println!("    - {:?}", at);
                    }
                } else {
                    println!("  Nenhum ataque aparente identificado");
                }
            } else {
                println!("  Snapshot não encontrado para o endereço {:?}", target);
            }
        }
    }

    Ok(())
}

