use crate::{
    tx_aggregator::{AnnotatedTx, TxAggregator, TxGroup},
    state_cache_manager::{SnapshotProfile, StateCacheManager},
    tx_nature_tagger::TxNatureTagger,
};
use dashmap::DashMap;
use ethernity_core::{traits::RpcProvider, error::Result, types::TransactionHash};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalMode {
    Normal,
    Burst,
    Recovery,
}

#[derive(Debug, Clone)]
struct BufferedTx {
    tx: AnnotatedTx,
    expires_at: Instant,
    first_window: u64,
}

#[derive(Debug, Clone)]
pub struct SyncMetadata {
    pub window_id: u64,
    pub block_shadowed: bool,
    pub evaluated_with_stale_state: bool,
    pub timestamp_drifted: bool,
    pub state_alignment_score: f64,
    pub timestamp_jitter_score: f64,
}

#[derive(Debug, Clone)]
pub struct GroupReady {
    pub group: TxGroup,
    pub metadata: SyncMetadata,
}

pub struct MempoolSupervisor<P> {
    provider: P,
    tagger: TxNatureTagger<P>,
    state_manager: StateCacheManager<P>,
    aggregator: TxAggregator,
    buffer: DashMap<TransactionHash, BufferedTx>,
    min_tx_count: usize,
    dt_max: Duration,
    max_active_groups: usize,
    operational_mode: OperationalMode,
    window_id: u64,
    last_block: u64,
    last_tick: Instant,
    window_duration: Duration,
    influx_counter: usize,
}

impl<P: RpcProvider + Clone> MempoolSupervisor<P> {
    pub fn new(provider: P, min_tx_count: usize, dt_max: Duration, max_active_groups: usize) -> Self {
        Self {
            tagger: TxNatureTagger::new(provider.clone()),
            state_manager: StateCacheManager::new(provider.clone()),
            aggregator: TxAggregator::new(),
            buffer: DashMap::new(),
            provider,
            min_tx_count,
            dt_max,
            max_active_groups,
            operational_mode: OperationalMode::Normal,
            window_id: 0,
            last_block: 0,
            last_tick: Instant::now(),
            window_duration: Duration::from_millis(500),
            influx_counter: 0,
        }
    }

    fn adaptive_ttl(&self, tx: &AnnotatedTx) -> Duration {
        match self.operational_mode {
            OperationalMode::Burst => Duration::from_secs(3),
            OperationalMode::Recovery => Duration::from_secs(7),
            OperationalMode::Normal => {
                if tx.gas_price > 100.0 { Duration::from_secs(3) } else { Duration::from_secs(5) }
            }
        }
    }

    pub fn ingest_tx(&mut self, tx: AnnotatedTx) {
        let ttl = self.adaptive_ttl(&tx);
        let expires_at = Instant::now() + ttl;
        self.buffer.insert(tx.tx_hash, BufferedTx { tx, expires_at, first_window: self.window_id });
        self.influx_counter += 1;
    }

    fn compute_state_alignment(&self, group: &TxGroup, current_block: u64) -> f64 {
        match group.block_number {
            Some(bn) if bn >= current_block.saturating_sub(1) => 1.0,
            Some(_) => 0.5,
            None => 0.8,
        }
    }

    fn compute_jitter(&self, group: &TxGroup) -> f64 {
        if group.txs.len() <= 1 { return 0.0; }
        let avg = group.txs.iter().map(|t| t.first_seen as f64).sum::<f64>() / group.txs.len() as f64;
        let var = group.txs.iter().map(|t| {
            let d = t.first_seen as f64 - avg;
            d * d
        }).sum::<f64>() / group.txs.len() as f64;
        var.sqrt()
    }

    async fn finalize_groups(&mut self, block_number: u64) -> Result<Vec<GroupReady>> {
        if self.aggregator.groups().is_empty() {
            return Ok(vec![]);
        }
        self.state_manager
            .snapshot_groups(self.aggregator.groups(), block_number, SnapshotProfile::Basic)
            .await?;
        let mut out = Vec::new();
        for g in self.aggregator.groups().values() {
            let align = self.compute_state_alignment(g, block_number);
            let jitter = self.compute_jitter(g);
            out.push(GroupReady {
                group: g.clone(),
                metadata: SyncMetadata {
                    window_id: self.window_id,
                    block_shadowed: false,
                    evaluated_with_stale_state: align < 0.6,
                    timestamp_drifted: jitter > 0.2,
                    state_alignment_score: align,
                    timestamp_jitter_score: jitter,
                },
            });
        }
        self.aggregator = TxAggregator::new();
        Ok(out)
    }

    pub async fn tick(&mut self) -> Result<Vec<GroupReady>> {
        let now = Instant::now();
        let block = self.provider.get_block_number().await?;
        let new_block = if block != self.last_block { Some(block) } else { None };
        self.last_block = block;

        // remove expired
        let mut remove = Vec::new();
        for item in self.buffer.iter() {
            if item.expires_at <= now {
                remove.push(*item.key());
            }
        }
        for h in remove { self.buffer.remove(&h); }

        // adapt mode
        let elapsed = now.duration_since(self.last_tick);
        let rate = self.influx_counter as f64 / elapsed.as_secs_f64().max(0.001);
        self.influx_counter = 0;
        self.last_tick = now;
        if rate > 50.0 { self.operational_mode = OperationalMode::Burst; self.window_duration = Duration::from_millis(500); }
        else { self.operational_mode = OperationalMode::Normal; self.window_duration = Duration::from_secs(2); }

        // move txs from buffer to aggregator
        for mut entry in self.buffer.iter_mut() {
            let mut tx = entry.tx.clone();
            if entry.first_window != self.window_id {
                tx.confidence *= 0.9; // penalização por sobreposição
                entry.first_window = self.window_id;
            }
            self.aggregator.add_tx(tx);
        }

        let mut result = Vec::new();
        if let Some(bn) = new_block {
            result = self.finalize_groups(bn).await?;
            self.window_id += 1;
        } else if self.aggregator.groups().len() >= self.max_active_groups {
            result = self.finalize_groups(block).await?;
            self.window_id += 1;
        }
        Ok(result)
    }
}
