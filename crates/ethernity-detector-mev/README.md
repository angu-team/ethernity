# ethernity-detector-mev

**Tipo:** Crate modular para detecção passiva de vítimas de MEV no mempool Ethereum

**Escopo:** Detecção multi-vítima, multi-token, multi-contexto, com inferência estática e leitura de estado determinística.

**Ambiente:** Tempo real, sem execução, simulação ou trace.

**Foco:** Reconhecimento de agrupamentos exploráveis com cálculo de slippage tolerada, convexidade e estimativa de backrun.

---

## ✦ Descrição Geral

`mev-victim-detector` é uma crate para uso em sistemas de monitoramento de mempool Ethereum, dedicada à **detecção passiva de oportunidades de exploração MEV** (e.g., sandwich, backrun, spoof) com múltiplas vítimas e tokens em paralelo.

A crate opera por inferência estática e leitura de estado on-chain sem qualquer forma de execução de transação. Sua arquitetura é orientada a fluxo contínuo de estado e centrada em *contextos econômicos isolados*, permitindo avaliação paralela de agrupamentos disjuntos em tempo real.

---

## ✦ Arquitetura de Módulos

### 1. **TxNatureTagger**

Classifica cada transação com base em `calldata` e `bytecode`. Extrai:

- `tags`: funções estimadas (swap, proxy, transfer)
- `token_paths`: sequência de tokens afetados
- `targets`: pools ou contratos relevantes

  → Primeira etapa do pipeline.


---

### 2. **TxAggregator**

Agrupa transações anotadas por `token_paths` + `targets`, formando:

- Janelas temporais com múltiplas vítimas
- Buckets paralelizáveis por `group_key`

  → Emite agrupamentos coesos para avaliação de impacto.


---

### 3. **StateSnapshotRepository**

Cache persistente de snapshots:

- Armazena em RocksDB por `(contract_address, block_number)`
- Valida `block_hash` para lidar com forks
- Evita recomputações redundantes

  → Fornece estado consistente com a cadeia vigente.


---

### 4. **StateImpactEvaluator**

Avalia agrupamentos por:

- Slippage tolerada (por vítima)
- Convexidade de impacto
- Estimativa de retorno no backrun

  → Gera `opportunity_score` por grupo com `expectedProfitBackrun`.


---

### 5. **AttackDetector**

Verifica se agrupamentos contêm:

- Frontruns externos
- Sandwich em formação
- Spoofing de preço
- Backruns evidentes

  → Marca agrupamentos contaminados para exclusão ou reclassificação.


---

### 6. **MempoolSupervisor**

Orquestra todo o ciclo:

- Regula ingestão de transações
- Controla janelas de agrupamento
- Sincroniza blocos com `blockNumber`
- Atua como **FSM assíncrona** guiada por eventos `SupervisorEvent`

```rust
enum SupervisorEvent {
    NewTxObserved(Tx),
    BlockAdvanced(BlockMetadata),
    StateRefreshed(String),
    GroupFinalized(String),
}
```

  → Garante consistência temporal e coordenação entre os módulos.


---

## ✦ Output Final

Por grupo analisado:

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
  "expectedProfitBackrun": 47.2,
  "attack_detected": false
  }
```

---

## ✦ Propriedades Operacionais

- **Não simula, não executa, não envia transações**
- Processamento **100% local e determinístico**
- Paralelização natural por agrupamento
- Tolerante a ruído de mempool e transações incompletas
- Capaz de operar em ambiente RPC limitado

---

## ✦ Casos de Uso

- **Bots MEV passivos** (filtro de agrupamentos viáveis)
- **Sistemas de defesa anti-MEV** (reconhecimento de sandbox hostil)
- **Dashboards de inteligência MEV**
- **Análise forense de transações mempool**

---

## ✦ Requisitos

- Acesso a um nó Ethereum RPC com suporte a:
    - `eth_call`
    - `eth_getCode`
    - `eth_getStorageAt`
- Buffer de transações pendentes (mempool stream)

---