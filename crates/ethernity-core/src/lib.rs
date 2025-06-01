/*!
 * Ethernity Core
 * 
 * Tipos e utilitários compartilhados para a workspace Ethernity
 */

pub mod types;
pub mod traits;
pub mod utils;
pub mod error;

// Re-exportações públicas
pub use error::Error;
pub use types::*;
