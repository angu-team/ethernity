# sandwich-victim

Biblioteca para detectar oportunidades de ataque *sandwich* em transações Ethereum. A análise executa a transação em um fork local da blockchain usando `anvil` e calcula:

- rota dos tokens trocados
- slippage real comparado com a cotação esperada
- quantidade mínima de tokens capaz de afetar o preço
- lucro potencial de uma estratégia de front‑run e back‑run
- identificação dinâmica do router envolvido (extraído exclusivamente dos logs da simulação)
- reconhecimento de todas as variações de funções de swap V2
- suporte ao PancakeSwap SmartRouterV3 com decodificação da multicall

A arquitetura segue o princípio de responsabilidade única. Cada módulo possui
uma função clara:
`core` contém o analisador de transações e cálculo de métricas,
`simulation` gerencia sessões reutilizáveis do Anvil,
`dex` provê decodificação e consultas on-chain,
`client` abstrai chamadas RPC com cache simples e
`types` define as estruturas de dados. Assim o código fica organizado e fácil de manter.

O código expõe funções assíncronas e pode ser extendido com novos métodos de avaliação.


Consulte o diretório [examples](./examples/) para um exemplo de uso via linha de
comando. O utilitário recebe um hash de transação e busca os dados em um node
RPC executando a simulação automaticamente com o `anvil`.
