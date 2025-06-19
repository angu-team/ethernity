# Exemplos - ethernity-finder

Este exemplo mostra como utilizar o `ethernity-finder` para localizar nodes Ethereum via Shodan.

```
cargo run --example find_nodes -- <CHAIN_ID> <LIMIT|all> <METHOD1,METHOD2,...>
```

O limite pode ser um número ou `all` para buscar todos os nodes válidos. A lista de métodos deve ser separada por vírgulas.

Métodos suportados:

- `debug_traceTransaction`
- `admin_nodeInfo`
- `admin_peers`
- `txpool_content`
- `trace_block`
