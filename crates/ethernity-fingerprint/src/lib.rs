//! Ethernity Fingerprint
//!
//! Provides deterministic semantic fingerprints for EVM smart contract bytecode.

pub mod utils;
pub mod parser;
pub mod cfg;
pub mod ir;
pub mod dispatcher;
pub mod fbs;
pub mod fingerprint;

pub use fingerprint::*;
