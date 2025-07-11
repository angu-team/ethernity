/*! Sandwich Victim
 *
 * Crate para detectar oportunidades de ataque do tipo sandwich em transações
 * Ethereum. Utiliza simulação local para estimar métricas de viabilidade.
 */

pub mod types;
pub mod simulation;
pub mod dex;
pub mod core;
pub mod filters;
pub mod log_semantics;
pub mod detectors;
