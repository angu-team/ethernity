use thiserror::Error;

/// Erros que podem ocorrer durante a simulação
#[derive(Debug, Error)]
pub enum SimulationError {
    /// Falha ao iniciar o processo do Anvil
    #[error("falha ao iniciar anvil: {0}")]
    AnvilSpawn(String),

    /// Falha ao criar provider conectado ao Anvil
    #[error("falha ao criar provider do anvil: {0}")]
    ProviderCreation(String),

    /// Erro ao enviar transação
    #[error("falha ao enviar transação: {0}")]
    SendTransaction(String),

    /// Erro ao aguardar resultado da transação
    #[error("falha ao aguardar transação: {0}")]
    AwaitTransaction(String),

    /// Operação realizada após o encerramento da sessão
    #[error("sessao ja encerrada")]
    SessionClosed,
}

/// Resultado padrão da crate
pub type Result<T> = std::result::Result<T, SimulationError>;
