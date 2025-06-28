use crate::types::TransactionData;
use crate::simulation::error::{Result, SimulationError};
use ethers::prelude::*;
use std::time::Duration;

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

/// Executa a transação em um fork local utilizando o Anvil
pub async fn simulate_transaction(
    config: &SimulationConfig,
    tx: &TransactionData,
) -> Result<SimulationOutcome> {
    use ethers::utils::Anvil;

    let mut anvil = Anvil::new().fork(config.rpc_endpoint.clone());
    if let Some(block) = config.block_number {
        anvil = anvil.fork_block_number(block);
    }
    let anvil = anvil.spawn();

    let provider = Provider::<Http>::try_from(anvil.endpoint())
        .map_err(|e| SimulationError::ProviderCreation(e.to_string()))?
        .interval(Duration::from_millis(1));

    provider
        .request::<_, ()>("anvil_impersonateAccount", [tx.from])
        .await
        .map_err(|e| SimulationError::ImpersonateAccount(e.to_string()))?;

    let tx_request = TransactionRequest::new()
        .from(tx.from)
        .to(tx.to)
        .data(tx.data.clone())
        .value(tx.value)
        .gas(tx.gas)
        .gas_price(tx.gas_price)
        .nonce(tx.nonce);

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
