/*! Sandwich Victim
 *
 * Crate para detectar oportunidades de ataque do tipo sandwich em transações
 * Ethereum. Utiliza simulação local para estimar métricas de viabilidade.
 */

pub mod types;
pub mod client {
    pub mod rpc;
    pub use rpc::*;
}
pub mod simulation;
pub mod dex;
pub mod core;
