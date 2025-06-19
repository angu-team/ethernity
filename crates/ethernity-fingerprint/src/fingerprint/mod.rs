use crate::{dispatcher, fbs};

/// Dispatcher entry mapping a selector to code offset.
#[derive(Debug, Clone)]
pub struct DispatchEntry {
    pub selector: [u8; 4],
    pub dest: usize,
}

/// Identified function information extracted from bytecode.
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub selector: [u8; 4],
    pub entry: usize,
    pub code: Vec<u8>,
}

/// Possible mutability classification for a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mutability { Pure, View, Mutative }

/// Resulting fingerprint for a function.
#[derive(Debug, Clone)]
pub struct FunctionFingerprint {
    pub selector: [u8; 4],
    pub mutability: Mutability,
    pub fbs_hash: [u8; 32],
    pub cfg_hash: [u8; 32],
    pub ir: Vec<String>,
}

pub use dispatcher::{extract_dispatch_entries, extract_functions, global_function_fingerprint};

pub use fbs::function_behavior_signature;
