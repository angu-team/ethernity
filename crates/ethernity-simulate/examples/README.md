# Exemplo - ethernity-simulate

Este exemplo demonstra como criar uma sessão de simulação a partir de um endpoint WebSocket, enviar uma transação simples e finalizar a sessão.

```bash
cargo run --example session_demo -- <RPC_WS_ENDPOINT>
```

Substitua `<RPC_WS_ENDPOINT>` pelo endereço RPC desejado (por exemplo, wss://mainnet.infura.io/ws/v3/YOUR_KEY). O `anvil` é iniciado com `--auto-impersonate` e o bloco inicial pode ser ajustado diretamente no exemplo.
