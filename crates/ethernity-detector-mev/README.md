# ethernity-detector-mev

**Tipo:** Crate modular para detecção passiva de vítimas de MEV no mempool Ethereum

**Escopo:** Detecção multi-vítima, multi-token e multi-contexto, usando inferência estática e leitura de estado determinística.

**Ambiente:** Tempo real, sem execução, simulação ou trace.

**Foco:** Reconhecimento de agrupamentos exploráveis com cálculo de slippage, convexidade e estimativa de backrun.

*Não inclui detecção de bots MEV concorrentes*

---

## ✦ Descrição Geral

`ethernity-detector-mev` oferece uma pipeline completa para inspeção de transações Ethereum em tempo real. O objetivo é identificar agrupamentos de potenciais vítimas de MEV e estimar o lucro possível em estratégias como frontrun, sandwich e backrun. Não há envio nem simulação de transações: toda a inferência é feita localmente a partir de dados do mempool e snapshots de estado on-chain.

---

## ✦ Arquitetura de Módulos

### 1. **TxNatureTagger**
Classifica cada transação com base no `calldata` e no bytecode do contrato destino.
- `tags` inferidas (swap-v2, swap-v3, transfer, proxy-call etc.)
- `token_paths` detectados no corpo da transação
- `targets` envolvidos

### 2. **TxAggregator**
Agrupa transações anotadas por par de tokens e alvos comuns, formando janelas paralelizáveis identificadas por `group_key`.
- Reavalia contaminação e capacidade de reordenação
- Mantém ordem temporal das transações observadas

### 3. **StateSnapshotRepository**
Repositório de snapshots com persistência em **redb**.
- Chaves de armazenamento `(contract_address, block_number)`
- Verificação de `block_hash` para lidar com forks
- Histórico curto para verificar volatilidade e backups/restauração opcionais

### 4. **StateImpactEvaluator**
Calcula impacto econômico esperado de um grupo.
- Slippage tolerada por vítima
- Histórico de slippage para média móvel
- Convexidade e simulação leve de curva constante ou Uniswap V3
- Produz `opportunity_score` e `expected_profit_backrun`

### 5. **RpcStateProvider**
Interface de acesso ao estado via RPC com cache embutido e fallback opcional.
- Métodos `reserves()` e `slot0()` com LRU cache
- Suporta múltiplos providers para tolerância a falhas

### 6. **MempoolSupervisor**
Orquestrador do ciclo completo.
- Ajusta TTL de transações e modo de operação (Normal/Burst/Recovery)
- Gere janelas de avaliação sincronizadas com a altura do bloco
- Emite `GroupReady` contendo agrupamento e metadados de sincronização

---

## ✦ Estratégia de Identificação

1. **Classificação** – cada transação é rotulada pelo `TxNatureTagger`.
2. **Agrupamento** – `TxAggregator` junta transações compatíveis em grupos.
3. **Snapshot de Estado** – `StateSnapshotRepository` coleta reserves e slot0 via `RpcStateProvider`.
4. **Avaliação de Impacto** – `StateImpactEvaluator` estima lucro e risco considerando slippage histórica e convexidade.
5. **Supervisão** – `MempoolSupervisor` coordena essas etapas, adaptando janelas conforme o volume (por exemplo, redes como BSC com blocos rápidos).

O resultado final inclui tokens analisados, métricas de slippage e um `opportunity_score` que indica a viabilidade econômica de executar um ataque MEV.

---

## ✦ Exemplo de Saída
```json
{
  "group_id": "0xUniswapPair_TokenA_TokenB_block17629811",
  "tokens": ["0xTokenA", "0xTokenB"],
  "victims": [
    {
      "tx_hash": "0xTx1",
      "slippage_tolerated": 9.24,
      "amountIn": 320.0,
      "expectedAmountOut": 385.6,
      "amountOutMin": 350.0
    }
  ],
  "opportunity_score": 0.83,
  "expectedProfitBackrun": 47.2
}
```

---

## ✦ Propriedades Operacionais
- Não simula nem envia transações
- Processamento local e determinístico
- Paralelização natural por agrupamento
- Robusto contra ruído do mempool
- Opera em ambientes RPC limitados (incluindo BSC)

---

## ✦ Casos de Uso
- Bots MEV passivos
- Sistemas de defesa anti-MEV
- Dashboards de inteligência
- Análises forenses de mempool

---

## ✦ Requisitos
- Acesso a um nó Ethereum RPC com suporte a:
  - `eth_call`
  - `eth_getCode`
  - `eth_getStorageAt`
- Stream de transações pendentes (mempool)
