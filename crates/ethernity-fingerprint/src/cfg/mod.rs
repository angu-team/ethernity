use std::collections::BTreeSet;

use crate::{fingerprint::FunctionInfo, parser::{Instruction, parse_instructions}, utils::{keccak256}};

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub start: usize,
    pub instrs: Vec<Instruction>,
    pub edges: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    pub blocks: Vec<BasicBlock>,
}

/// Builds a simple control flow graph for the given function.
pub fn build_cfg(func: &FunctionInfo) -> ControlFlowGraph {
    let insts = parse_instructions(&func.code);
    let mut boundaries = BTreeSet::new();
    boundaries.insert(0usize);
    for ins in &insts {
        if ins.opcode == 0x5b && ins.pos != 0 {
            boundaries.insert(ins.pos);
        }
    }
    for ins in &insts {
        match ins.opcode {
            0x56 | 0x57 | 0xf3 | 0xfd | 0xfe | 0xff => {
                if let Some(next) = insts.iter().find(|i| i.pos > ins.pos) {
                    boundaries.insert(next.pos);
                }
            }
            _ => {}
        }
    }
    let mut points: Vec<usize> = boundaries.into_iter().collect();
    points.sort();

    let mut blocks = Vec::new();
    for (idx, start) in points.iter().enumerate() {
        let end = points.get(idx + 1).copied().unwrap_or(func.code.len());
        let slice: Vec<Instruction> = insts
            .iter()
            .filter(|ins| ins.pos >= *start && ins.pos < end)
            .cloned()
            .collect();
        blocks.push(BasicBlock { start: *start, instrs: slice, edges: Vec::new() });
    }

    for i in 0..blocks.len() {
        let mut edges = Vec::new();
        if let Some(ins) = blocks[i].instrs.last() {
            match ins.opcode {
                0x56 | 0x57 => {
                    if let Some(prev) = blocks[i]
                        .instrs
                        .iter()
                        .rev()
                        .find(|p| (0x60..=0x7f).contains(&p.opcode))
                    {
                        let mut dest = 0usize;
                        for &b in &prev.data {
                            dest = (dest << 8) | (b as usize);
                        }
                        if let Some(idx) = blocks.iter().position(|b| b.start == dest) {
                            edges.push(idx);
                        }
                    }
                    if ins.opcode == 0x57 && i + 1 < blocks.len() {
                        edges.push(i + 1);
                    }
                }
                0xf3 | 0xfd | 0xfe | 0xff => {}
                _ => {
                    if i + 1 < blocks.len() {
                        edges.push(i + 1);
                    }
                }
            }
        }
        blocks[i].edges = edges;
    }
    ControlFlowGraph { blocks }
}

/// Hashes only the structure of the CFG (blocks and edges).
pub fn hash_cfg_structure(cfg: &ControlFlowGraph) -> [u8; 32] {
    let mut repr = String::new();
    for (idx, block) in cfg.blocks.iter().enumerate() {
        let mut edges = block.edges.clone();
        edges.sort();
        let edges_str: Vec<String> = edges.iter().map(|e| e.to_string()).collect();
        repr.push_str(&format!("{}:{}|", idx, edges_str.join(",")));
    }
    keccak256(repr.as_bytes())
}
