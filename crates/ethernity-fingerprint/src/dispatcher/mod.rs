use crate::{fingerprint::{DispatchEntry, FunctionInfo}, utils::keccak256};

/// Extracts dispatcher entries from contract bytecode.
/// Looks for `PUSH4` selector followed by `EQ` and a jump destination.
pub fn extract_dispatch_entries(bytecode_hex: &str) -> Vec<DispatchEntry> {
    let bytes = hex::decode(bytecode_hex.trim_start_matches("0x")).unwrap_or_default();
    let mut entries = Vec::new();
    let mut i = 0;
    while i + 9 < bytes.len() {
        if bytes[i] == 0x63 && bytes[i + 5] == 0x14 {
            let mut selector = [0u8; 4];
            selector.copy_from_slice(&bytes[i + 1..i + 5]);
            let op = bytes[i + 6];
            if (0x60..=0x61).contains(&op) {
                let n = (op - 0x60 + 1) as usize;
                if i + 7 + n <= bytes.len() && bytes[i + 7 + n] == 0x57 {
                    let mut dest: usize = 0;
                    for b in &bytes[i + 7..i + 7 + n] {
                        dest = (dest << 8) | (*b as usize);
                    }
                    entries.push(DispatchEntry { selector, dest });
                    i += 7 + n;
                    continue;
                }
            }
        }
        i += 1;
    }
    entries
}

/// Calculates the Global Function Fingerprint for the given bytecode.
pub fn global_function_fingerprint(bytecode_hex: &str) -> [u8; 32] {
    let mut selectors: Vec<[u8; 4]> = extract_dispatch_entries(bytecode_hex)
        .into_iter()
        .map(|e| e.selector)
        .collect();
    selectors.sort();
    let mut concat = Vec::new();
    for s in selectors { concat.extend_from_slice(&s); }
    keccak256(&concat)
}

/// Extracts individual functions from the bytecode using dispatcher entries.
pub fn extract_functions(bytecode_hex: &str) -> Vec<FunctionInfo> {
    let bytes = hex::decode(bytecode_hex.trim_start_matches("0x")).unwrap_or_default();
    let entries = extract_dispatch_entries(bytecode_hex);
    let mut dests: Vec<usize> = entries.iter().map(|e| e.dest).collect();
    dests.sort();
    dests.dedup();

    let mut functions = Vec::new();
    for (idx, dest) in dests.iter().enumerate() {
        let start = *dest;
        let end = dests.get(idx + 1).copied().unwrap_or(bytes.len());
        if start >= bytes.len() || start >= end { continue; }
        let code = bytes[start..end].to_vec();
        if let Some(sel) = entries.iter().find(|e| e.dest == *dest).map(|e| e.selector) {
            functions.push(FunctionInfo { selector: sel, entry: start, code });
        }
    }
    functions
}
