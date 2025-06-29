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

## Monitorar o mempool via WebSocket

Este exemplo conecta-se a um endpoint RPC WebSocket e escuta as transações pendentes do mempool. Cada transação é analisada e, se houver indícios de que seja uma potencial vítima de *sandwich*, as métricas são exibidas no console.

```bash
cargo run -p sandwich-victim --example mempool_watch -- <WS_RPC_ENDPOINT>
```

Certifique-se de utilizar um node completo que ofereça o método `newPendingTransactions` via WebSocket.
