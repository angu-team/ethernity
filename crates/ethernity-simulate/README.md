# ethernity-simulate

Crate responsável por criar e gerenciar ambientes de simulação de transações em forks da blockchain.

A implementação inicial utiliza o **Anvil** para spawnar forks locais, mas a arquitetura foi preparada para aceitar novos provedores de simulação no futuro.

Principais funcionalidades:
- Criação de sessões de fork baseadas em um RPC remoto e bloco específico.
- Envio de transações simuladas e obtenção do `TransactionReceipt`.
- Encerramento manual das sessões e limpeza automática por timeout.
- Inicialização do `anvil` com o argumento `--auto-impersonate`.
- Possibilidade de definir opcionalmente o bloco inicial do fork.
