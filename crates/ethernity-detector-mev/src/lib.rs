/*!
 * Ethernity Detector MEV
 *
 * Módulo inicial `TxNatureTagger` para inferência estática
 * de transações Ethereum observadas na mempool.
 */

mod tx_nature_tagger;
mod tx_aggregator;

pub use tx_nature_tagger::*;
pub use tx_aggregator::*;
