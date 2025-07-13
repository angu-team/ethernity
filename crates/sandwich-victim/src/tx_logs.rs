use ethereum_types::H256;
use ethers::prelude::Log;

/// Conjunto de logs de uma transação ou execução externa
#[derive(Debug, Clone)]
pub struct TxLogs {
    pub tx_hash: Option<H256>,
    pub logs: Vec<Log>,
}

impl TxLogs {
    /// Retorna os logs decodificados utilizando mapeamentos de eventos conhecidos
    pub fn decoded_logs(&self) -> Vec<crate::log_semantics::MappedLog> {
        crate::log_semantics::map_logs(&self.logs)
    }
}
