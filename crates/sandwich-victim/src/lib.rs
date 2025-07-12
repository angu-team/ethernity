/*! Sandwich Victim
 *
 * Crate para detectar oportunidades de ataque do tipo sandwich em transações
 * Ethereum. A análise trabalha apenas com logs recebidos como entrada,
 * sem realizar qualquer simulação local.
 */

pub mod core;
pub mod detectors;
pub mod dex;
pub mod filters;
pub mod log_semantics;
pub mod tx_logs;
pub mod types;
