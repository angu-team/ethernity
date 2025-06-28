# Exemplos - sandwich-victim

Este exemplo demonstra como utilizar a crate para analisar uma transação e verificar se ela pode ser vítima de um ataque *sandwich*.

Execute o exemplo informando o endpoint RPC e o hash da transação que deseja analisar:

```bash
cargo run -p sandwich-victim --example analyze_tx --features anvil -- <RPC_ENDPOINT> <TX_HASH>
```

O programa obtém os dados da transação via RPC, executa-a em um fork local com o `anvil` e imprime as métricas calculadas.
