/*!
 * Ethernity Detector MEV
 *
 * Módulo inicial `TxNatureTagger` para inferência estática
 * de transações Ethereum observadas na mempool.
 */

mod tx_nature_tagger;
mod tx_aggregator;
mod state_impact_evaluator;
mod state_cache_manager;
mod attack_detector;

pub use tx_nature_tagger::*;
pub use tx_aggregator::*;
pub use state_impact_evaluator::*;
pub use state_cache_manager::*;
pub use attack_detector::*;
