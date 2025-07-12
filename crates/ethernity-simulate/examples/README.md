# Exemplo - ethernity-simulate

Este diretório contém exemplos de como utilizar a crate para criar sessões de simulação.

### session_demo
Cria uma sessão de fork a partir de um endpoint WebSocket e envia uma transação simples entre contas desbloqueadas do Anvil.

```bash
cargo run --example session_demo -- <RPC_WS_ENDPOINT>
```

Substitua `<RPC_WS_ENDPOINT>` pelo endereço RPC desejado (por exemplo, `wss://mainnet.infura.io/ws/v3/YOUR_KEY`).

### simulate_tx
Simula uma transação existente obtida via hash. Aceita tanto endpoints HTTP quanto WebSocket.

```bash
cargo run --example simulate_tx -- <RPC_ENDPOINT> <TX_HASH>
```

O exemplo conecta-se ao endpoint informado, recupera a transação e executa a simulação em uma sessão baseada no bloco da própria transação, exibindo o tempo total gasto.
