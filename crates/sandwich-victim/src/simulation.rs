use crate::types::{TransactionData};
use anyhow::{Result, anyhow};
use ethers::prelude::*;
#[cfg(feature = "anvil")]
use std::time::Duration;

/// Configurações para a simulação local
#[derive(Debug, Clone)]
pub struct SimulationConfig {
    pub rpc_endpoint: String,
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
    #[cfg(feature = "anvil")]
    {
        use ethers::utils::Anvil;

        let anvil = Anvil::new()
            .fork(config.rpc_endpoint.clone())
            .spawn();

        let provider = Provider::<Http>::try_from(anvil.endpoint())?
            .interval(Duration::from_millis(1));

        provider
            .request::<_, ()>("anvil_impersonateAccount", [tx.from])
            .await?;

        let tx_request = TransactionRequest::new()
            .from(tx.from)
            .to(tx.to)
            .data(tx.data.clone())
            .value(tx.value)
            .gas(tx.gas)
            .gas_price(tx.gas_price);

        let pending = provider.send_transaction(tx_request, None).await?;
        let receipt = pending
            .await?
            .ok_or_else(|| anyhow!("transação não minerada"))?;
        Ok(SimulationOutcome {
            tx_hash: Some(receipt.transaction_hash),
            logs: receipt.logs,
        })
    }
    #[cfg(not(feature = "anvil"))]
    {
        let _ = config;
        let _ = tx;
        Err(anyhow!("anvil feature not enabled"))
    }
}
