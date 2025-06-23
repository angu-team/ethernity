/*!
 * Ethernity Detector MEV
 *
 * Módulo inicial `TxNatureTagger` para inferência estática
 * de transações Ethereum observadas na mempool.
 */

mod tx_nature_tagger;
mod tx_aggregator;
mod state_impact_evaluator;
mod state_snapshot_repository;
mod attack_detector;
mod mempool_supervisor;
mod events;
mod traits;
mod rpc_state_provider;

pub use tx_nature_tagger::*;
pub use tx_aggregator::*;
pub use state_impact_evaluator::*;
pub use state_snapshot_repository::*;
pub use attack_detector::*;
pub use mempool_supervisor::*;
pub use events::*;
pub use traits::*;
pub use rpc_state_provider::*;
