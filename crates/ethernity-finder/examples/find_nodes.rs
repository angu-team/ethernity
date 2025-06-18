use std::env;

use ethernity_finder::{FinderOptions, NodeFinder, ShodanFinder};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Uso: {} <CHAIN_ID> <LIMIT|all> <METHOD1,METHOD2,...>", args[0]);
        std::process::exit(1);
    }

    let chain_id = args[1].parse::<u64>()?;
    let limit = if args[2].to_lowercase() == "all" {
        None
    } else {
        Some(args[2].parse::<usize>()?)
    };
    let methods = args[3]
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let finder = ShodanFinder::new();
    let opts = FinderOptions {
        chain_id,
        methods,
        limit,
    };

    let nodes = finder.find_nodes(opts).await?;
    for node in nodes {
        println!("Node {}:{} (chainId {})", node.ip, node.port, node.chain_id);
        for method in node.methods {
            if method.success {
                println!("  - {}: OK", method.method);
            } else if let Some(err) = method.error {
                println!("  - {}: ERROR ({})", method.method, err);
            } else {
                println!("  - {}: UNSUPPORTED", method.method);
            }
        }
    }
    Ok(())
}
