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

A implementação opera 100% offline e não depende de metadados ou fonte do
contrato. O CFG é construído a partir do bytecode e cada bloco é interpretado
para gerar uma IR semântica canônica, resistente a reorganizações e ruído
superficial.
