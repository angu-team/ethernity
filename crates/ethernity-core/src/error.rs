use thiserror::Error;

/// Erros comuns da biblioteca Ethernity
#[derive(Error, Debug)]
pub enum Error {
    /// Erro de comunicação com o node Ethereum
    #[error("Erro de RPC: {0}")]
    RpcError(String),
    
    /// Erro de decodificação de dados
    #[error("Erro de decodificação: {0}")]
    DecodeError(String),
    
    /// Erro de codificação de dados
    #[error("Erro de codificação: {0}")]
    EncodeError(String),
    
    /// Erro de validação
    #[error("Erro de validação: {0}")]
    ValidationError(String),
    
    /// Erro de timeout
    #[error("Timeout: {0}")]
    TimeoutError(String),
    
    /// Recurso não encontrado
    #[error("Não encontrado: {0}")]
    NotFound(String),
    
    /// Erro genérico
    #[error("{0}")]
    Other(String),
}

/// Tipo de resultado usado em toda a biblioteca
pub type Result<T> = std::result::Result<T, Error>;
