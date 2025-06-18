# Exemplo de Detecção de Eventos - Rust

Este exemplo demonstra como utilizar o `ethernity-deeptrace` para analisar uma transação específica e executar os detectores de eventos disponíveis.

## Uso

```bash
cargo run --example event_detector -- <RPC_ENDPOINT> <TX_HASH>
```

- `<RPC_ENDPOINT>`: URL do node Ethereum (HTTP ou WebSocket).
- `<TX_HASH>`: Hash da transação a ser analisada.

O programa exibirá na saída padrão os eventos identificados para a transação informada ou avisará caso nada seja encontrado.

