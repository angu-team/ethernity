# sandwich-victim

Biblioteca para detectar oportunidades de ataque *sandwich* em transações Ethereum. A análise executa a transação em um fork local da blockchain usando `anvil` e calcula:

- rota dos tokens trocados
- slippage real comparado com a cotação esperada
- quantidade mínima de tokens capaz de afetar o preço
- lucro potencial de uma estratégia de front‑run e back‑run
- identificação dinâmica do router envolvido
- reconhecimento de todas as variações de funções de swap V2

A arquitetura segue o princípio de responsabilidade única. Cada módulo possui
uma função clara:
`simulation` executa a transação em um fork local, `dex` traz utilidades para
roteadores e funções de swap, `analysis` coleta métricas on-chain e `types` guarda
as estruturas de dados. Assim o código fica organizado e fácil de manter.

O código expõe funções assíncronas e pode ser extendido com novos métodos de avaliação.


Consulte o diretório [examples](./examples/) para um exemplo de uso via linha de
comando. O utilitário recebe um hash de transação e busca os dados em um node
RPC executando a simulação automaticamente com o `anvil`.
