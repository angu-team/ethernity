# Exemplos - sandwich-victim

Este diretório contém um pequeno utilitário de linha de comando para
analisar uma transação e verificar se ela é potencial vítima de um ataque
*sandwich*.

Primeiro crie um arquivo JSON descrevendo a transação que deseja inspecionar.
O formato segue a estrutura de `TransactionData` da biblioteca, por exemplo:

```json
{
  "from": "0x...",
  "to": "0x...",
  "data": "0x...",
  "value": "0x0",
  "gas": 21000,
  "gas_price": "0x0",
  "nonce": "0x0"
}
```

Em seguida execute o exemplo informando o endpoint RPC e o caminho do arquivo.
Não se esqueça de habilitar a feature `anvil`:

```bash
cargo run -p sandwich-victim --example analyze_tx --features anvil -- <RPC_ENDPOINT> <arquivo.json>
```

O programa carrega os dados do arquivo, executa a transação em um fork local
com o `anvil` e imprime as métricas calculadas.
