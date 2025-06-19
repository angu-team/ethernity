# ethernity-fingerprint

Crate responsável pela geração de assinaturas determinísticas para contratos EVM.
Inclui dois algoritmos principais:

- **Global Function Fingerprint (GFF)**: resume a superfície pública do contrato
  através do hash dos selectors extraídos do dispatcher.
- **Function Behavior Signature (FBS)**: tenta representar semanticamente o
  comportamento de cada função pública a partir do controle de fluxo. O CFG é
  percorrido em BFS para cobrir todos os ramos e ciclos, com detecção de loops e
  inferência de mutabilidade. A IR produzida é ordenada
  deterministicamente para garantir hashes reprodutíveis e gera dois hashes:
  um da própria IR e outro somente da estrutura do CFG.

Durante essa interpretação a mutabilidade (Pure, View ou Mutative) é inferida
a partir dos opcodes observados. Leituras de armazenamento (`SLOAD`) elevam a
classificação para *View*. Instruções que podem modificar o estado, como
`SSTORE`, `CALL`, `CALLCODE`, `DELEGATECALL`, `CREATE`, `CREATE2` ou
`SELFDESTRUCT`, marcam a função como *Mutative*. Por outro lado, `STATICCALL`
é tratado como operação somente leitura, não alterando o estado e mantendo a
classificação no máximo em *View*.

A implementação opera 100% offline e não depende de metadados ou fonte do
contrato. O CFG é construído a partir do bytecode e cada bloco é interpretado
para gerar uma IR semântica canônica, resistente a reorganizações e ruído
superficial.
