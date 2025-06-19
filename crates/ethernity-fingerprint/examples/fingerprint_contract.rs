use std::env;

use ethernity_fingerprint::{extract_functions, global_function_fingerprint, function_behavior_signature};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <BYTECODE_HEX>", args[0]);
        std::process::exit(1);
    }
    let bytecode = &args[1];

    let gff = global_function_fingerprint(bytecode);
    println!("Global Function Fingerprint: 0x{}", hex::encode(gff));

    for func in extract_functions(bytecode) {
        let fp = function_behavior_signature(&func);
        println!("\nSelector: 0x{}", hex::encode(fp.selector));
        println!("Mutability: {:?}", fp.mutability);
        println!("FBS hash: 0x{}", hex::encode(fp.fbs_hash));
        println!("CFG hash: 0x{}", hex::encode(fp.cfg_hash));
        if !fp.ir.is_empty() {
            println!("IR:");
            for stmt in fp.ir {
                println!("  {}", stmt);
            }
        }
    }
    Ok(())
}
