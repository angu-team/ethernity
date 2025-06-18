# Exemplos - Rust

Aqui estão exemplos de como utilizar o `ethernity-deeptrace` para analisar transações Ethereum.

## Detecção de Eventos

```bash
cargo run --example event_detector -- <RPC_ENDPOINT> <TX_HASH>
```

Analisa a transação indicada e executa os detectores de eventos disponíveis, mostrando as ocorrências encontradas (reentrancy, sandwich attack etc.).

## Detecção de Padrões

```bash
cargo run --example pattern_detector -- <RPC_ENDPOINT> <TX_HASH>
```

Executa a análise completa e lista os padrões DeFi (flash loans, arbitragem, rug pull e outros) detectados para a transação informada.

