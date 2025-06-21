/*!
 * Ethernity DeepTrace
 *
 * Biblioteca para análise profunda de transações EVM via call traces.
 * Permite rastreamento detalhado de fluxo de fundos, detecção de padrões
 * e análise de interações entre contratos.
 */

mod memory;
mod trace;
pub mod analyzer;
mod patterns;
mod utils;
mod config;
mod types;
mod deeptrace;

pub use analyzer::*;
// Re-exportações públicas
pub use memory::*;
pub use patterns::*;
pub use trace::*;
pub use utils::*;
pub use config::*;
pub use types::*;
pub use deeptrace::*;
