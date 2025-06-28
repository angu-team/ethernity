# Exemplos - sandwich-victim

Este diretório contém um pequeno utilitário de linha de comando para
analisar uma transação e verificar se ela é potencial vítima de um ataque
*sandwich*.

Informe um endpoint RPC e o hash de uma transação já incluída em bloco. O
utilitário buscará os dados necessários no node e executará a análise.

```bash
cargo run -p sandwich-victim --example analyze_tx -- <RPC_ENDPOINT> <TX_HASH>
```

O programa obtém os dados da transação e a executa em um fork local com o
`anvil`, imprimindo as métricas calculadas.
