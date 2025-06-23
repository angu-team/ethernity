# ethernity-detector-mev

Crate dedicada à detecção passiva de oportunidades de MEV a partir de transações observadas na mempool. O módulo inicial `TxNatureTagger` realiza inferência estática da natureza de uma transação analisando o `calldata` e o bytecode do endereço de destino. O resultado é um conjunto de tags e informações resumidas que podem ser usadas por outros componentes do sistema para agrupamento e priorização.
