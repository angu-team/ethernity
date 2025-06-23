use ethernity_core::types::TransactionHash;
use ethereum_types::{Address, H256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tiny_keccak::{Hasher, Keccak};

/// Transação anotada proveniente do `TxNatureTagger` com informações adicionais de mempool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotatedTx {
    pub tx_hash: TransactionHash,
    pub token_paths: Vec<Address>,
    pub targets: Vec<Address>,
    pub tags: Vec<String>,
    pub first_seen: u64,
    pub gas_price: f64,
    pub max_priority_fee_per_gas: Option<f64>,
    pub confidence: f64,
}

/// Grupo de transações correlacionadas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxGroup {
    pub group_key: H256,
    pub token_paths: Vec<Address>,
    pub targets: Vec<Address>,
    pub txs: Vec<AnnotatedTx>,
    pub block_number: Option<u64>,
    pub direction_signature: String,
    pub ordering_certainty_score: f64,
    pub reorderable: bool,
    pub contaminated: bool,
}

/// Agrupador de transações relevates de acordo com tags e caminhos de tokens.
pub struct TxAggregator {
    groups: HashMap<H256, TxGroup>,
}

impl TxAggregator {
    /// Cria um novo agregador vazio.
    pub fn new() -> Self {
        Self { groups: HashMap::new() }
    }

    /// Obtém referência aos grupos existentes.
    pub fn groups(&self) -> &HashMap<H256, TxGroup> {
        &self.groups
    }

    /// Adiciona uma transação ao agregador, retornando a chave do grupo resultante.
    pub fn add_tx(&mut self, tx: AnnotatedTx) -> Option<H256> {
        if !Self::passes_filter(&tx) {
            return None;
        }

        let key = Self::group_key(&tx.token_paths, &tx.targets, &tx.tags);
        let group = self.groups.entry(key).or_insert_with(|| TxGroup {
            group_key: key,
            token_paths: tx.token_paths.clone(),
            targets: tx.targets.clone(),
            txs: Vec::new(),
            block_number: None,
            direction_signature: Self::direction_signature(&tx.token_paths),
            ordering_certainty_score: 1.0,
            reorderable: false,
            contaminated: false,
        });

        if tx.confidence < 0.5 {
            let high = group
                .txs
                .iter()
                .filter(|t| t.confidence >= 0.5)
                .count();
            if high < 2 {
                return None;
            }
        }

        group.txs.push(tx);
        group.txs.sort_by(|a, b| {
            if a.first_seen == b.first_seen {
                a.gas_price
                    .partial_cmp(&b.gas_price)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.first_seen.cmp(&b.first_seen)
            }
        });

        group.ordering_certainty_score = Self::calc_ordering_certainty(group);
        group.reorderable = group.ordering_certainty_score < 0.6;
        group.contaminated = Self::calc_contamination(group);

        Some(key)
    }

    fn passes_filter(tx: &AnnotatedTx) -> bool {
        if tx.token_paths.len() < 2 {
            return false;
        }
        if tx.targets.is_empty() {
            return false;
        }
        let allowed = ["swap-v2", "swap-v3", "token-move", "router-call"];
        tx.tags.iter().any(|t| allowed.contains(&t.as_str()))
    }

    fn tags_signature(tags: &[String]) -> String {
        let mut sorted = tags.to_vec();
        sorted.sort();
        sorted.join(":")
    }

    fn group_key(token_paths: &[Address], targets: &[Address], tags: &[String]) -> H256 {
        let mut bytes = Vec::new();
        for addr in token_paths {
            bytes.extend_from_slice(addr.as_bytes());
        }
        for addr in targets {
            bytes.extend_from_slice(addr.as_bytes());
        }
        let sig = Self::tags_signature(tags);
        bytes.extend_from_slice(sig.as_bytes());
        let mut keccak = Keccak::v256();
        keccak.update(&bytes);
        let mut out = [0u8; 32];
        keccak.finalize(&mut out);
        H256::from(out)
    }

    fn direction_signature(paths: &[Address]) -> String {
        paths
            .iter()
            .map(|a| format!("0x{:x}", a))
            .collect::<Vec<_>>()
            .join("→")
    }

    fn calc_ordering_certainty(group: &TxGroup) -> f64 {
        if group.txs.len() <= 1 {
            return 1.0;
        }
        let first = group.txs.first().unwrap().first_seen as f64;
        let last = group.txs.last().unwrap().first_seen as f64;
        let delta = last - first;
        if delta <= 30.0 { 1.0 } else { 0.7 }
    }

    fn calc_contamination(group: &TxGroup) -> bool {
        if group.txs.is_empty() {
            return false;
        }
        let avg = group.txs.iter().map(|t| t.confidence).sum::<f64>() / group.txs.len() as f64;
        let var = group
            .txs
            .iter()
            .map(|t| {
                let d = t.confidence - avg;
                d * d
            })
            .sum::<f64>()
            / group.txs.len() as f64;
        var.sqrt() > 0.2
    }
}

