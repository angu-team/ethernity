use std::env;
use std::str::FromStr;
use std::sync::Arc;

use ethernity_core::types::TransactionHash;
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use ethernity_deeptrace::{DeepTraceAnalyzer, TraceAnalysisConfig, detectors::{DetectorManager, DetectedEvent}, analyzer::TraceAnalysisResult};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Leitura simples dos argumentos de linha de comando
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Uso: {} <RPC_ENDPOINT> <TX_HASH>", args[0]);
        std::process::exit(1);
    }

    let endpoint = &args[1];
    let tx_hash = TransactionHash::from_str(&args[2])?;

    // Configura o cliente RPC
    let rpc_config = RpcConfig { endpoint: endpoint.clone(), ..Default::default() };
    let rpc_client = Arc::new(EthernityRpcClient::new(rpc_config).await?);

    // Instancia o analisador principal
    let analyzer = DeepTraceAnalyzer::new(rpc_client.clone(), Some(TraceAnalysisConfig::default()));

    println!("🔍 Analisando transação {tx_hash:?}...");
    let tx_analysis = analyzer.analyze_transaction(tx_hash).await.expect("ERR");

    // Constrói o resultado de análise para os detectores
    let analysis_result = TraceAnalysisResult {
        call_tree: tx_analysis.call_tree.clone(),
        token_transfers: tx_analysis.token_transfers.clone(),
        contract_creations: tx_analysis.contract_creations.clone(),
        execution_path: tx_analysis.execution_path.clone(),
    };

    // Executa todos os detectores disponíveis
    let detector_manager = DetectorManager::new();
    let events = detector_manager.detect_all(&analysis_result).await.expect("ERR");

    if events.is_empty() {
        println!("Nenhum evento suspeito detectado para a transação {tx_hash:?}.");
    } else {
        println!("Eventos detectados:");
        for DetectedEvent { event_type, description, confidence, .. } in events {
            println!("- {event_type}: {description} (confiança {:.2}%)", confidence * 100.0);
        }
    }

    Ok(())
}
