use crate::simulation::SimulationOutcome;
use ethers::types::H256;
use std::str::FromStr;

/// Trait para filtros de resultados de simulação
pub trait Filter: Send + Sync {
    /// Aplica o filtro ao resultado.
    /// Retorna `Some` quando a simulação deve continuar no pipeline
    /// ou `None` para descartar.
    fn apply(&self, outcome: SimulationOutcome) -> Option<SimulationOutcome>;
}

/// Pipeline de filtros a serem executados sequencialmente
#[derive(Default)]
pub struct FilterPipeline {
    filters: Vec<Box<dyn Filter>>,
}

impl FilterPipeline {
    /// Cria pipeline vazio
    pub fn new() -> Self {
        Self { filters: Vec::new() }
    }

    /// Adiciona um filtro ao pipeline
    pub fn push<F: Filter + 'static>(mut self, filter: F) -> Self {
        self.filters.push(Box::new(filter));
        self
    }

    /// Executa os filtros em sequência retornando o resultado final
    pub fn run(&self, mut outcome: SimulationOutcome) -> Option<SimulationOutcome> {
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

const SWAP_TOPIC_V2: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
const SWAP_TOPIC_V3: &str = "0x19b47279256b2a23a1665c810c8d55a1758940ee09377d4f8d26497a3577dc83";

impl Filter for SwapLogFilter {
    fn apply(&self, outcome: SimulationOutcome) -> Option<SimulationOutcome> {
        let topic_v2 = H256::from_str(SWAP_TOPIC_V2).expect("valid topic hex");
        let topic_v3 = H256::from_str(SWAP_TOPIC_V3).expect("valid topic hex");
        if outcome
            .logs
            .iter()
            .any(|log| log.topics.get(0) == Some(&topic_v2) || log.topics.get(0) == Some(&topic_v3))
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

    fn outcome_with_topics(topics: Vec<H256>) -> SimulationOutcome {
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
        SimulationOutcome { tx_hash: None, logs: vec![log] }
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

