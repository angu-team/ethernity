use thiserror::Error;

/// Erros que podem ocorrer durante a simulação com o Anvil
#[derive(Error, Debug)]
pub enum SimulationError {
    /// Falha ao iniciar o processo do Anvil
    #[error("falha ao iniciar anvil: {0}")]
    AnvilSpawn(String),
    /// Falha ao criar provider conectado ao Anvil
    #[error("falha ao criar provider do anvil: {0}")]
    ProviderCreation(String),
    /// Erro ao tentar impersonar a conta de envio
    #[error("falha ao impersonar conta: {0}")]
    ImpersonateAccount(String),
    /// Erro ao enviar a transação de simulação
    #[error("falha ao enviar transação: {0}")]
    SendTransaction(String),
    /// Erro ao aguardar pela mineração
    #[error("erro ao aguardar mineração: {0}")]
    AwaitMining(String),
    /// A transação não foi minerada durante a simulação
    #[error("transação não minerada")]
    TransactionNotMined,
}

/// Resultado padrão da simulação
pub type Result<T> = std::result::Result<T, SimulationError>;
