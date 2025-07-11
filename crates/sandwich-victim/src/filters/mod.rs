use crate::tx_logs::TxLogs;
use ethers::types::H256;
use std::str::FromStr;

/// Trait para filtros de resultados de simulação
pub trait Filter: Send + Sync {
    /// Aplica o filtro ao resultado.
    /// Retorna `Some` quando a simulação deve continuar no pipeline
    /// ou `None` para descartar.
    fn apply(&self, outcome: TxLogs) -> Option<TxLogs>;
}

/// Pipeline de filtros a serem executados sequencialmente
#[derive(Default)]
pub struct FilterPipeline {
    filters: Vec<Box<dyn Filter>>,
}

impl FilterPipeline {
    /// Cria pipeline vazio
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Adiciona um filtro ao pipeline
    pub fn push<F: Filter + 'static>(mut self, filter: F) -> Self {
        self.filters.push(Box::new(filter));
        self
    }

    /// Executa os filtros em sequência retornando o resultado final
    pub fn run(&self, mut outcome: TxLogs) -> Option<TxLogs> {
        for f in &self.filters {
            match f.apply(outcome) {
                Some(out) => outcome = out,
                None => return None,
            }
        }
        Some(outcome)
    }
}

/// Filtro que verifica a presença do evento `Swap` nos logs
pub struct SwapLogFilter;

const SWAP_TOPIC: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";

impl Filter for SwapLogFilter {
    fn apply(&self, outcome: TxLogs) -> Option<TxLogs> {
        let topic = H256::from_str(SWAP_TOPIC).expect("valid topic hex");
        if outcome
            .logs
            .iter()
            .any(|log| log.topics.get(0) == Some(&topic))
        {
            Some(outcome)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::{Address, Bytes, Log};

    fn outcome_with_topics(topics: Vec<H256>) -> TxLogs {
        let log = Log {
            address: Address::zero(),
            topics,
            data: Bytes::default(),
            block_hash: None,
            block_number: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        };
        TxLogs {
            tx_hash: None,
            logs: vec![log],
        }
    }

    #[test]
    fn filter_passes_when_topic_present() {
        let outcome = outcome_with_topics(vec![H256::from_str(SWAP_TOPIC).unwrap()]);
        let pipeline = FilterPipeline::new().push(SwapLogFilter);
        assert!(pipeline.run(outcome).is_some());
    }

    #[test]
    fn filter_discards_when_topic_absent() {
        let outcome = outcome_with_topics(vec![H256::zero()]);
        let pipeline = FilterPipeline::new().push(SwapLogFilter);
        assert!(pipeline.run(outcome).is_none());
    }
}
