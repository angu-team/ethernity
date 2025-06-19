use std::collections::{HashSet, VecDeque};
use ethereum_types::U256;

use crate::{cfg::{build_cfg, hash_cfg_structure}, fingerprint::{FunctionFingerprint, FunctionInfo, Mutability}, ir::{canonicalize_commutative, stmt_to_string, Expr, Stmt}, utils::keccak256};

/// Interprets EVM bytecode building a simplified semantic IR and infers mutability.
fn interpret_function(func: &FunctionInfo) -> (Vec<String>, Mutability) {
    let cfg = build_cfg(func);
    let mut ir = Vec::new();
    let mut mutability = Mutability::Pure;
    let mut visited: HashSet<usize> = HashSet::new();
    let mut queue: VecDeque<(usize, Vec<Expr>)> = VecDeque::new();
    queue.push_back((0, Vec::new()));

    while let Some((idx, mut stack)) = queue.pop_front() {
        if !visited.insert(idx) {
            ir.push("LOOP".to_string());
            continue;
        }
        let block = &cfg.blocks[idx];
        for ins in &block.instrs {
            match ins.opcode {
                0x60..=0x7f => {
                    let mut val = U256::zero();
                    for b in &ins.data { val = (val << 8) + U256::from(*b); }
                    stack.push(Expr::Const(val));
                }
                0x35 => { // CALLDATALOAD
                    if let Some(off) = stack.pop() {
                        match off {
                            Expr::Const(v) => {
                                let idx = (v.as_usize() / 32) as u32;
                                stack.push(Expr::Arg(idx));
                            }
                            other => stack.push(other),
                        }
                    }
                }
                0x54 => { // SLOAD
                    if mutability < Mutability::View { mutability = Mutability::View; }
                    if let Some(slot) = stack.pop() { stack.push(Expr::SLoad(Box::new(slot))); }
                }
                0x55 => { // SSTORE
                    mutability = Mutability::Mutative;
                    if let (Some(val), Some(slot)) = (stack.pop(), stack.pop()) {
                        ir.push(stmt_to_string(&Stmt::Write(slot.clone(), val.clone())));
                    }
                }
                0x01 => { // ADD
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        let (a,b) = canonicalize_commutative(a,b);
                        stack.push(Expr::Add(Box::new(a), Box::new(b)));
                    }
                }
                0x03 => { // SUB
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        stack.push(Expr::Sub(Box::new(a), Box::new(b)));
                    }
                }
                0x02 => { // MUL
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        let (a,b) = canonicalize_commutative(a,b);
                        stack.push(Expr::Mul(Box::new(a), Box::new(b)));
                    }
                }
                0x14 => { // EQ
                    if let (Some(b), Some(a)) = (stack.pop(), stack.pop()) {
                        let (a,b) = canonicalize_commutative(a,b);
                        stack.push(Expr::Eq(Box::new(a), Box::new(b)));
                    }
                }
                0x57 => { // JUMPI
                    if let (Some(dest), Some(cond)) = (stack.pop(), stack.pop()) {
                        ir.push(stmt_to_string(&Stmt::Require(cond)));
                        if let Expr::Const(d) = dest {
                            if let Some(idx) = cfg.blocks.iter().position(|b| b.start == d.as_usize()) {
                                queue.push_back((idx, stack.clone()));
                            }
                        }
                    }
                }
                0xf1 | 0xf2 | 0xf4 | 0xfa | 0xff => { mutability = Mutability::Mutative; }
                0xfd => ir.push(stmt_to_string(&Stmt::Revert)),
                0xf3 => { if let Some(val) = stack.pop() { ir.push(stmt_to_string(&Stmt::Return(val))); } }
                _ => {}
            }
        }
        for &edge in &block.edges { queue.push_back((edge, stack.clone())); }
    }
    (ir, mutability)
}

fn hash_ir(ir: &[String]) -> [u8; 32] {
    let joined = ir.join(";").replace(' ', "");
    keccak256(joined.as_bytes())
}

pub fn function_behavior_signature(func: &FunctionInfo) -> FunctionFingerprint {
    let (mut ir, mutability) = interpret_function(func);
    ir.sort();
    let cfg = build_cfg(func);
    let hash = hash_ir(&ir);
    let cfg_hash = hash_cfg_structure(&cfg);
    FunctionFingerprint { selector: func.selector, mutability, fbs_hash: hash, cfg_hash, ir }
}
