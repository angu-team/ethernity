use crate::simulation::error::{Result, SimulationError};
use crate::types::TransactionData;
use ethers::prelude::*;
use std::time::Duration;
use url::Url;

/// Configurações para a simulação local
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    pub rpc_endpoint: String,
    pub block_number: Option<u64>,
}

/// Resultado simples da simulação
#[derive(Debug, Clone)]
pub struct SimulationOutcome {
    pub tx_hash: Option<H256>,
    pub logs: Vec<Log>,
}

impl SimulationOutcome {
    /// Retorna os logs decodificados de acordo com os mapeamentos semânticos
    pub fn decoded_logs(&self) -> Vec<crate::log_semantics::MappedLog> {
        crate::log_semantics::map_logs(&self.logs)
    }
}

/// Executa a transação em um fork local utilizando o Anvil
pub async fn simulate_transaction(
    config: &SimulationConfig,
    tx: &TransactionData,
) -> Result<SimulationOutcome> {
    use ethers::utils::Anvil;

    let mut anvil = Anvil::new()
        .fork("http://148.251.183.245:8545")
        .args(&[
            "--auto-impersonate".to_string(),
            // "--no-mining".to_string(),
            // "--gas-price=0".to_string(),
            // "--base-fee=0".to_string(),
            // "--gas-limit=30000000".to_string(),      // Limite de gás fixo e alto

        ]) // Define o limite de gás
        // .path("/home/moinho/.foundry/bin/anvil"); // Caminho para o binário do Anvil
        .path("/root/.foundry/bin/anvil");
    if let Some(block) = config.block_number {
        anvil = anvil.fork_block_number(block);
    }
    let anvil = anvil.spawn();

    let provider = Provider::<Http>::connect(&anvil.endpoint()).await;
    // let provider = Provider::<Http>::connect(&anvil.endpoint()).await;

    // provider
    //     .request::<_, ()>("anvil_impersonateAccount", [tx.from])
    //     .await
    //     .map_err(|e| SimulationError::ImpersonateAccount(e.to_string()))?;

    let tx_request = TransactionRequest::new()
        .from(tx.from)
        .to(tx.to)
        .data(tx.data.clone())
        .value(tx.value)
        .gas(tx.gas)
        .gas_price(tx.gas_price);

    let pending = provider
        .send_transaction(tx_request, None)
        .await
        .map_err(|e| SimulationError::SendTransaction(e.to_string()))?;
    let receipt = pending
        .await
        .map_err(|e| SimulationError::AwaitMining(e.to_string()))?
        .ok_or(SimulationError::TransactionNotMined)?;
    
    Ok(SimulationOutcome {
        tx_hash: Some(receipt.transaction_hash),
        logs: receipt.logs,
    })
}
