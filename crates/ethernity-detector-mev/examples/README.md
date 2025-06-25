# Exemplos - ethernity-detector-mev

Estes exemplos demonstram formas de utilizar a crate para detectar oportunidades MEV.

Execute o exemplo apontando para um RPC que permita obter os blocos com transações completas:

```bash
cargo run --example example -- <RPC_ENDPOINT> [BLOCO]
```

`RPC_ENDPOINT` deve apontar para um node Ethereum (mainnet ou testnet). Opcionalmente informe o número do bloco a ser analisado. Se omitido, o bloco atual é utilizado.

### Monitorar a mempool em tempo real

```bash
cargo run --example mempool_monitor -- <ENDPOINT_WS>
```

Para esta variante é necessário um endpoint **WebSocket** que ofereça `eth_subscribe` para as transações pendentes. O programa acompanha a mempool e exibe grupos e ataques potenciais conforme são detectados.
