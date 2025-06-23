use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use ethereum_types::Address;
use ethernity_core::types::TransactionHash;
use crate::{AnnotatedTx, TxGroup, StateSnapshot, GroupImpact, AttackVerdict};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTx {
    pub tx_hash: TransactionHash,
    pub to: Address,
    pub input: Vec<u8>,
    pub first_seen: u64,
    pub gas_price: f64,
    pub max_priority_fee_per_gas: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEvent {
    pub group: TxGroup,
    pub snapshots: HashMap<Address, StateSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactEvent {
    pub group: TxGroup,
    pub impact: GroupImpact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatEvent {
    pub verdict: AttackVerdict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMetadata {
    pub number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupervisorEvent {
    NewTxObserved(AnnotatedTx),
    BlockAdvanced(BlockMetadata),
    StateRefreshed(String),
    GroupFinalized(String),
}

/// Simple event bus wrapper over [`tokio::sync::mpsc`] channels.
pub struct EventBus<T> {
    sender: mpsc::Sender<T>,
}

impl<T> EventBus<T> {
    pub fn new(capacity: usize) -> (Self, mpsc::Receiver<T>) {
        let (tx, rx) = mpsc::channel(capacity);
        (Self { sender: tx }, rx)
    }

    pub fn sender(&self) -> mpsc::Sender<T> {
        self.sender.clone()
    }
}
