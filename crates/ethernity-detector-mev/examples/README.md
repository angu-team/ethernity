# Exemplos - ethernity-detector-mev

Este exemplo demonstra o uso básico da crate para detectar oportunidades MEV em um bloco Ethereum.

Execute:

```bash
cargo run --example example -- <RPC_ENDPOINT> [BLOCO]
```

`RPC_ENDPOINT` deve apontar para um node Ethereum (mainnet ou testnet). Opcionalmente informe o número do bloco a ser analisado. Se omitido, o bloco atual é utilizado.
