# Exemplos - ethernity-detector-mev

Este exemplo demonstra o uso básico da crate para detectar oportunidades MEV em um bloco Ethereum.

Execute o exemplo apontando para um RPC que permita obter os blocos com transações completas:

```bash
cargo run --example example -- <RPC_ENDPOINT> [BLOCO]
```

`RPC_ENDPOINT` deve apontar para um node Ethereum (mainnet ou testnet). Opcionalmente informe o número do bloco a ser analisado. Se omitido, o bloco atual é utilizado.

## Escutando a mempool em tempo real

O exemplo `mempool_stream` demonstra como consumir transações pendentes de um nó Ethereum via WebSocket e alimentá-las no `MempoolSupervisor`.

Execute utilizando um endpoint WebSocket válido:

```bash
cargo run --example mempool_stream -- <WS_RPC_ENDPOINT>
```

Durante a execução, novos grupos de transações formados serão impressos no console conforme observados.
