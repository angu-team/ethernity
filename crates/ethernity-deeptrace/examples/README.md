# Exemplos - Rust

Aqui estão exemplos de como utilizar o `ethernity-deeptrace` para analisar transações Ethereum.

## Detecção de Padrões

```bash
cargo run --example pattern_detector -- <RPC_ENDPOINT> <TX_HASH>
```

Executa a análise completa e lista os padrões DeFi (flash loans, arbitragem, rug pull e outros) detectados para a transação informada.

