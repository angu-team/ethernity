use async_trait::async_trait;
use ethernity_core::{error::Result, types::TransactionHash};
use ethereum_types::{Address, U256};

/// Interface para obtenção de estado de contratos.
#[async_trait]
pub trait StateProvider: Send + Sync {
    /// Retorna as reserves (token0, token1) de um par.
    async fn reserves(&self, address: Address) -> Result<(U256, U256)>;

    /// Retorna informações do slot0 (sqrtPriceX96, liquidez).
    async fn slot0(&self, address: Address) -> Result<(U256, U256)>;
}

/// Predição de tag da transação.
#[derive(Debug, Clone)]
pub struct TagPrediction {
    pub tag: String,
    pub confidence: f64,
}

/// Classificador de transações com pontuação probabilística.
#[async_trait]
pub trait TransactionClassifier: Send + Sync {
    async fn classify(
        &self,
        to: Address,
        input: &[u8],
        tx_hash: TransactionHash,
    ) -> Result<Vec<TagPrediction>>;
}

/// Modelo de impacto econômico.
pub trait ImpactModel: Send + Sync {
    fn evaluate_group(
        &self,
        group: &crate::TxGroup,
        victims: &[crate::VictimInput],
        snapshot: &crate::StateSnapshot,
    ) -> crate::GroupImpact;
}
