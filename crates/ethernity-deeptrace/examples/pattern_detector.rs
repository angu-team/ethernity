use std::env;
use std::str::FromStr;
use std::sync::Arc;

use ethernity_core::traits::RpcProvider;
use ethernity_core::types::TransactionHash;
use ethernity_rpc::{EthernityRpcClient, RpcConfig};
use ethernity_deeptrace::{DeepTraceAnalyzer, TraceAnalysisConfig};

/// Analisa uma transação específica e exibe os padrões detectados.
async fn analyze_patterns(tx_hash: TransactionHash, rpc: Arc<dyn RpcProvider>) -> anyhow::Result<()> {
    // Inicializa o analisador com configurações padrão
    let analyzer = DeepTraceAnalyzer::new(rpc, Some(TraceAnalysisConfig::default()));

    // Processa a transação fornecida
    let result = analyzer.analyze_transaction(tx_hash).await.expect("ERR");

    // Exibe os padrões encontrados de maneira clara
    if result.detected_patterns.is_empty() {
        println!("Nenhum padrão encontrado para {tx_hash:?}");
    } else {
        println!("Padrões detectados para {tx_hash:?}:");
        for pattern in result.detected_patterns {
            println!(
                "- {:?}: {} (confiança {:.2}%)",
                pattern.pattern_type,
                pattern.description,
                pattern.confidence * 100.0
            );
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Leitura simples dos argumentos da linha de comando
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Uso: {} <RPC_ENDPOINT> <TX_HASH>", args[0]);
        std::process::exit(1);
    }

    let endpoint = &args[1];
    let tx_hash = TransactionHash::from_str(&args[2])?;

    // Configura o cliente RPC informado
    let rpc_config = RpcConfig { endpoint: endpoint.clone(), ..Default::default() };
    let rpc_client = Arc::new(EthernityRpcClient::new(rpc_config).await?);

    // Executa a análise de padrões
    analyze_patterns(tx_hash, rpc_client).await
}

