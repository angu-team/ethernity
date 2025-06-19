use ethernity_fingerprint::{fingerprint::{FunctionInfo, function_behavior_signature, Mutability}};

#[test]
fn test_fbs_deterministic() {
    let code = vec![0x60, 0x01, 0x60, 0x00, 0x55, 0x00];
    let func = FunctionInfo { selector: [0u8; 4], entry: 0, code };
    let fp1 = function_behavior_signature(&func);
    let fp2 = function_behavior_signature(&func);
    assert_eq!(fp1.fbs_hash, fp2.fbs_hash);
    assert_eq!(fp1.cfg_hash, fp2.cfg_hash);
}

#[test]
fn test_staticcall_classified_as_view() {
    let code = vec![0xfa, 0x00];
    let func = FunctionInfo { selector: [0u8; 4], entry: 0, code };
    let fp = function_behavior_signature(&func);
    assert_eq!(fp.mutability, Mutability::View);
}

#[test]
fn test_create_marks_mutative() {
    let code = vec![0xf0, 0x00];
    let func = FunctionInfo { selector: [0u8; 4], entry: 0, code };
    let fp = function_behavior_signature(&func);
    assert_eq!(fp.mutability, Mutability::Mutative);
}
