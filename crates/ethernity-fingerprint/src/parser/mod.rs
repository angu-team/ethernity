/// Decoded instruction from bytecode.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: u8,
    pub data: Vec<u8>,
    pub pos: usize,
}

/// Decodes raw bytecode into a list of instructions.
pub fn parse_instructions(code: &[u8]) -> Vec<Instruction> {
    let mut instructions = Vec::new();
    let mut i = 0;
    while i < code.len() {
        let opcode = code[i];
        let mut data = Vec::new();
        let pos = i;
        if (0x60..=0x7f).contains(&opcode) {
            let n = (opcode - 0x60 + 1) as usize;
            let end = core::cmp::min(i + 1 + n, code.len());
            data.extend_from_slice(&code[i + 1..end]);
            i += 1 + n;
        } else {
            i += 1;
        }
        instructions.push(Instruction { opcode, data, pos });
    }
    instructions
}
