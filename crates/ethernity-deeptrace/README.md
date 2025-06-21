# ethernity-deeptrace

**Análise profunda de transações EVM via call traces**

## Visão Geral

O `ethernity-deeptrace` é uma biblioteca especializada para análise profunda de transações Ethereum através de call traces. Fornece capacidades avançadas de rastreamento de fluxo de fundos, detecção de padrões suspeitos, análise de interações entre contratos e identificação de atividades maliciosas.

## Características Principais

- 🔍 **Análise de Call Traces**: Decompõe traces complexos em estruturas navegáveis
- 🌳 **Árvore de Chamadas**: Representação hierárquica de todas as interações
- 💰 **Análise de Fluxo de Fundos**: Rastreamento detalhado de transferências de tokens
- 🤖 **Análise MEV**: Detecção de atividades de Maximal Extractable Value
- 📊 **Detecção de Padrões**: Sistema extensível de detectores especializados
- 🧠 **Gerenciamento de Memória**: Otimizações para análise de traces grandes
- ⚡ **Processamento Paralelo**: Análise concorrente quando possível

## Estrutura do Projeto

```
ethernity-deeptrace/
├── src/
│   ├── lib.rs           # Interface principal e tipos públicos
│   ├── analyzer.rs      # Analisador principal de traces
│   ├── trace.rs         # Estruturas de trace e call tree
│   ├── patterns/        # Módulos de detectores de padrões DeFi
│   ├── memory.rs        # Gerenciamento de memória e cache
│   └── utils.rs         # Utilitários para análise
├── Cargo.toml           # Dependências e metadados
└── README.md
```

## Dependências Principais

- **ethernity-core**: Tipos e traits compartilhadas
- **ethernity-rpc**: Cliente RPC para obter dados
- **ethers**: Biblioteca Ethereum para Rust
- **ethereum-types**: Tipos básicos do Ethereum
- **serde**: Serialização e deserialização
- **tokio**: Runtime assíncrono
- **lru**: Cache LRU eficiente
- **dashmap**: HashMap concorrente
- **parking_lot**: Primitivas de sincronização
- **hex**: Codificação/decodificação hexadecimal

---

## ⚙️ Configuração

### TraceAnalysisConfig

```rust
#[derive(Debug, Clone)]
pub struct TraceAnalysisConfig {
    /// Profundidade máxima de análise recursiva
    pub max_depth: usize,
    
    /// Limite de memória em bytes
    pub memory_limit: usize,
    
    /// Timeout para análise em milissegundos
    pub timeout_ms: u64,
    
    /// Habilita cache de resultados intermediários
    pub enable_cache: bool,
    
    /// Habilita análise paralela quando possível
    pub enable_parallel: bool,
    
    /// Configuração de detecção de padrões
    pub pattern_detection: PatternDetectionConfig,
}
```

### PatternDetectionConfig

```rust
#[derive(Debug, Clone)]
pub struct PatternDetectionConfig {
    /// Habilita detecção de padrões de token ERC20
    pub detect_erc20: bool,
}
```

### Configurações Padrão

```rust
impl Default for TraceAnalysisConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            memory_limit: 100 * 1024 * 1024, // 100 MB
            timeout_ms: 30000, // 30 segundos
            enable_cache: true,
            enable_parallel: true,
            pattern_detection: PatternDetectionConfig::default(),
        }
    }
}
```

### Configurações Personalizadas

```rust
use ethernity_deeptrace::*;

// Configuração para análise intensiva
let intensive_config = TraceAnalysisConfig {
    max_depth: 20,
    memory_limit: 500 * 1024 * 1024, // 500 MB
    timeout_ms: 120000, // 2 minutos
    enable_cache: true,
    enable_parallel: true,
    pattern_detection: PatternDetectionConfig {
        detect_erc20: true,
    },
};

// Configuração para análise rápida
let fast_config = TraceAnalysisConfig {
    max_depth: 5,
    memory_limit: 50 * 1024 * 1024, // 50 MB
    timeout_ms: 5000, // 5 segundos
    enable_cache: false,
    enable_parallel: false,
    pattern_detection: PatternDetectionConfig { detect_erc20: true },
};

// Configuração para detecção de segurança
let security_config = TraceAnalysisConfig {
    max_depth: 15,
    memory_limit: 200 * 1024 * 1024, // 200 MB
    timeout_ms: 60000, // 1 minuto
    enable_cache: true,
    enable_parallel: true,
    pattern_detection: PatternDetectionConfig { detect_erc20: true },
};
```

---

## 🔍 Analisador Principal

### DeepTraceAnalyzer

O componente central que coordena toda a análise.

```rust
use ethernity_deeptrace::*;
use ethernity_rpc::*;
use ethernity_core::types::*;
use std::sync::Arc;

// Criar o analisador
let rpc_client = Arc::new(EthernityRpcClient::new(rpc_config).await?);
let config = TraceAnalysisConfig::default();
let analyzer = DeepTraceAnalyzer::new(rpc_client, Some(config));
```

### Análise de Transação Única

```rust
// Analisar uma transação específica
let tx_hash = TransactionHash::from_str("0x...")?;
let analysis = analyzer.analyze_transaction(tx_hash).await?;

// Informações básicas
println!("📊 Análise da Transação {}", tx_hash);
println!("┌─ Status: {}", if analysis.status { "✅ Sucesso" } else { "❌ Falha" });
println!("├─ Bloco: {}", analysis.block_number);
println!("├─ Gas usado: {}", analysis.gas_used);
println!("├─ De: {}", analysis.from);
println!("├─ Para: {:?}", analysis.to);
println!("├─ Valor: {} ETH", analysis.value);
println!("├─ Chamadas: {}", analysis.call_tree.total_calls());
println!("├─ Profundidade máxima: {}", analysis.call_tree.max_depth());
println!("├─ Transferências de token: {}", analysis.token_transfers.len());
println!("├─ Contratos criados: {}", analysis.contract_creations.len());
└─ Padrões detectados: {}", analysis.detected_patterns.len());
```

### Análise em Lote

```rust
// Analisar múltiplas transações
let tx_hashes = vec![
    TransactionHash::from_str("0x...")?,
    TransactionHash::from_str("0x...")?,
    TransactionHash::from_str("0x...")?,
];

let batch_results = analyzer.analyze_batch(&tx_hashes).await?;

println!("📊 Resultados da Análise em Lote:");
for (i, result) in batch_results.iter().enumerate() {
    println!("Transação {}: {} padrões detectados", i + 1, result.detected_patterns.len());
    
    // Verificar padrões críticos
    let critical_patterns: Vec<_> = result.detected_patterns.iter()
        .filter(|p| p.confidence > 0.8)
        .collect();
    
    if !critical_patterns.is_empty() {
        println!("  ⚠️ {} padrões críticos encontrados", critical_patterns.len());
    }
}
```

---

## 🌳 Análise de Call Tree

### Estrutura da Árvore de Chamadas

```rust
// Acessar a árvore de chamadas
let call_tree = &analysis.call_tree;

// Estatísticas básicas
println!("🌳 Análise da Árvore de Chamadas:");
println!("├─ Total de chamadas: {}", call_tree.total_calls());
println!("├─ Profundidade máxima: {}", call_tree.max_depth());
println!("└─ Nó raiz: {:?} -> {:?}", call_tree.root.from, call_tree.root.to);
```

### Navegação pela Árvore

```rust
// Percorrer todos os nós
call_tree.traverse_preorder(|node| {
    println!("Chamada na profundidade {}: {:?} -> {:?}", 
        node.depth, node.from, node.to);
    
    if let Some(error) = &node.error {
        println!("  ❌ Erro: {}", error);
    }
    
    if node.value > U256::zero() {
        println!("  💰 Valor: {} wei", node.value);
    }
});

// Obter nós em uma profundidade específica
let depth_2_nodes = call_tree.nodes_at_depth(2);
println!("Nós na profundidade 2: {}", depth_2_nodes.len());

// Encontrar chamadas falhadas
let failed_calls = call_tree.failed_calls();
println!("Chamadas que falharam: {}", failed_calls.len());

for failed_call in failed_calls {
    println!("❌ Falha: {:?} -> {:?}", failed_call.from, failed_call.to);
    if let Some(error) = &failed_call.error {
        println!("   Erro: {}", error);
    }
}
```

### Análise por Endereço

```rust
// Analisar chamadas para um endereço específico
let target_address = Address::from_str("0x...")?;
let calls_to_target = call_tree.calls_to_address(&target_address);
let calls_from_target = call_tree.calls_from_address(&target_address);

println!("📞 Análise de Interações com {}", target_address);
println!("├─ Chamadas recebidas: {}", calls_to_target.len());
println!("└─ Chamadas enviadas: {}", calls_from_target.len());

// Analisar tipos de chamadas
for call in &calls_to_target {
    match call.call_type {
        CallType::Call => println!("  📞 CALL: {} gas", call.gas_used),
        CallType::StaticCall => println!("  🔍 STATICCALL: {} gas", call.gas_used),
        CallType::DelegateCall => println!("  🔄 DELEGATECALL: {} gas", call.gas_used),
        CallType::Create => println!("  🏭 CREATE: {} gas", call.gas_used),
        CallType::Create2 => println!("  🏭 CREATE2: {} gas", call.gas_used),
        _ => println!("  ❓ Outro tipo: {} gas", call.gas_used),
    }
}
```

---

## 💰 Análise de Transferências

### Transferências de Tokens

```rust
// Analisar transferências de tokens
println!("💰 Transferências de Tokens:");

for (i, transfer) in analysis.token_transfers.iter().enumerate() {
    match transfer.token_type {
        TokenType::Erc20 => {
            println!("{}. ERC20 Transfer", i + 1);
            println!("   Token: {}", transfer.token_address);
            println!("   De: {}", transfer.from);
            println!("   Para: {}", transfer.to);
            println!("   Valor: {}", transfer.amount);
        },
        TokenType::Erc721 => {
            println!("{}. NFT Transfer", i + 1);
            println!("   Token: {}", transfer.token_address);
            println!("   De: {}", transfer.from);
            println!("   Para: {}", transfer.to);
            if let Some(token_id) = transfer.token_id {
                println!("   Token ID: {}", token_id);
            }
        },
        _ => {
            println!("{}. Token Transfer (tipo desconhecido)", i + 1);
        }
    }
}
```

### Análise de Fluxo de Valor

```rust
use ethernity_deeptrace::utils::*;

// Analisar fluxo de valor
let value_flow = ValueFlowAnalyzer::analyze_value_flow(&analysis.token_transfers);

println!("📊 Análise de Fluxo de Valor:");
println!("├─ Endereços envolvidos: {}", value_flow.total_addresses);
println!("├─ Volume total: {}", value_flow.total_volume);

// Maiores recebedores
println!("🔝 Maiores Recebedores:");
for (i, (address, amount)) in value_flow.net_receivers.iter().take(5).enumerate() {
    println!("  {}. {}: {}", i + 1, address, amount);
}

// Maiores enviadores
println!("📤 Maiores Enviadores:");
for (i, (address, amount)) in value_flow.net_senders.iter().take(5).enumerate() {
    println!("  {}. {}: {}", i + 1, address, amount);
}

// Detectar padrões suspeitos
let suspicious_patterns = ValueFlowAnalyzer::detect_suspicious_patterns(&value_flow);
for pattern in suspicious_patterns {
    match pattern {
        SuspiciousPattern::HighConcentration { address, concentration } => {
            println!("⚠️ Alta concentração: {} recebeu {:.1}% do volume total", 
                address, concentration * 100.0);
        },
        SuspiciousPattern::CircularFlow { address1, address2, amount } => {
            println!("🔄 Fluxo circular suspeito entre {} e {}: {}", 
                address1, address2, amount);
        },
        _ => {}
    }
}
```

---

## 🏭 Análise de Criação de Contratos

### Contratos Criados

```rust
// Analisar contratos criados
println!("🏭 Contratos Criados:");

for (i, creation) in analysis.contract_creations.iter().enumerate() {
    println!("{}. Novo Contrato", i + 1);
    println!("   Criador: {}", creation.creator);
    println!("   Endereço: {}", creation.contract_address);
    println!("   Tipo: {:?}", creation.contract_type);
    println!("   Tamanho do init code: {} bytes", creation.init_code.len());
    
    // Analisar tipo de contrato
    match creation.contract_type {
        ContractType::Erc20Token => {
            println!("   🪙 Token ERC20 detectado");
        },
        ContractType::Erc721Token => {
            println!("   🖼️ Token ERC721 (NFT) detectado");
        },
        ContractType::DexPool => {
            println!("   🔄 Pool DEX detectado");
        },
        ContractType::Proxy => {
            println!("   🔄 Contrato Proxy detectado");
        },
        ContractType::Factory => {
            println!("   🏭 Factory Contract detectado");
        },
        ContractType::Unknown => {
            println!("   ❓ Tipo de contrato desconhecido");
        },
        _ => {}
    }
}
```

### Análise de Bytecode

```rust
use ethernity_deeptrace::utils::BytecodeAnalyzer;

// Analisar bytecode dos contratos criados
for creation in &analysis.contract_creations {
    if !creation.init_code.is_empty() {
        println!("🔍 Análise de Bytecode para {}", creation.contract_address);
        
        // Extrair seletores de função
        let selectors = BytecodeAnalyzer::extract_function_selectors(&creation.init_code);
        println!("   Seletores encontrados: {}", selectors.len());
        
        // Analisar complexidade
        let complexity = BytecodeAnalyzer::analyze_complexity(&creation.init_code);
        println!("   Score de complexidade: {:.2}", complexity.complexity_score());
        println!("   Operações de armazenamento: {}", complexity.storage_ops);
        println!("   Operações de sistema: {}", complexity.system_ops);
        
        // Detectar padrões de proxy
        let proxy_patterns = BytecodeAnalyzer::detect_proxy_patterns(&creation.init_code);
        if !proxy_patterns.is_empty() {
            println!("   Padrões de proxy detectados: {:?}", proxy_patterns);
        }
    }
}
```

---

## ⚡ Análise de Gas

### Análise de Uso de Gas

```rust
use ethernity_deeptrace::utils::GasAnalyzer;

// Analisar uso de gas
let gas_analysis = GasAnalyzer::analyze_gas_usage(&analysis.execution_path);

println!("⛽ Análise de Gas:");
println!("├─ Gas total usado: {}", gas_analysis.total_gas_used);
println!("├─ Operações: {}", gas_analysis.operation_count);
println!("├─ Gas por operação:");
println!("│  ├─ CALL: {}", gas_analysis.call_gas);
println!("│  ├─ STATICCALL: {}", gas_analysis.static_call_gas);
println!("│  ├─ DELEGATECALL: {}", gas_analysis.delegate_call_gas);
println!("│  ├─ CREATE: {}", gas_analysis.create_gas);
println!("│  └─ CREATE2: {}", gas_analysis.create2_gas);
└─ Operações caras: {}", gas_analysis.expensive_operations.len());

// Analisar operações caras
for expensive_op in &gas_analysis.expensive_operations {
    println!("💸 Operação cara:");
    println!("   Tipo: {:?}", expensive_op.call_type);
    println!("   De: {}", expensive_op.from);
    println!("   Para: {}", expensive_op.to);
    println!("   Gas usado: {}", expensive_op.gas_used);
    println!("   Profundidade: {}", expensive_op.depth);
}

// Detectar anomalias de gas
let gas_anomalies = GasAnalyzer::detect_gas_anomalies(&gas_analysis);
for anomaly in gas_anomalies {
    match anomaly {
        GasAnomaly::ExcessiveGasUsage { total_gas } => {
            println!("⚠️ Uso excessivo de gas: {}", total_gas);
        },
        GasAnomaly::TooManyExpensiveOperations { count } => {
            println!("⚠️ Muitas operações caras: {}", count);
        },
        GasAnomaly::HighDelegateCallRatio { ratio } => {
            println!("⚠️ Proporção alta de DELEGATECALL: {:.1}%", ratio * 100.0);
        },
        _ => {}
    }
}
```

---

## 🧠 Gerenciamento de Memória

### Monitoramento de Memória

```rust
use ethernity_deeptrace::memory::memory::*;

// Obter estatísticas de memória
let memory_stats = analyzer.memory_stats();
println!("🧠 Estatísticas de Memória:");

for (cache_name, stats) in memory_stats.cache_stats {
    println!("Cache '{}': {} entradas", cache_name, stats.entries);
    println!("  Taxa de acerto: {:.1}%", stats.hit_ratio * 100.0);
}

for (pool_name, stats) in memory_stats.buffer_pool_stats {
    println!("Pool '{}': {} alocações", pool_name, stats.allocations);
    println!("  Taxa de reutilização: {:.1}%", stats.reuse_ratio * 100.0);
}
```

### Cache Personalizado

```rust
use std::time::Duration;

// Criar cache personalizado
let cache: SmartCache<String, String> = SmartCache::new(1000, Duration::from_secs(300));

// Inserir dados
cache.insert("trace_123".to_string(), "cached_data".to_string());

// Recuperar dados
if let Some(data) = cache.get(&"trace_123".to_string()) {
    println!("Dados do cache: {}", data);
}

// Estatísticas do cache
let stats = cache.stats();
println!("Cache hits: {}", stats.hits);
println!("Cache misses: {}", stats.misses);
```

### Monitor de Memória

```rust
// Criar monitor de memória
let memory_manager = Arc::new(MemoryManager::new());
let monitor = MemoryMonitor::new(
    memory_manager,
    Duration::from_secs(1), // Intervalo de amostragem
    1000 // Máximo de entradas no histórico
);

// Iniciar monitoramento
monitor.start_monitoring().await?;

// Executar análise intensiva
for i in 0..100 {
    let tx_hash = generate_random_tx_hash();
    let _ = analyzer.analyze_transaction(tx_hash).await;
}

// Obter histórico
let history = monitor.get_history();
for snapshot in history.iter().take(10) {
    println!("Timestamp: {:?}", snapshot.timestamp);
    println!("Memória usada: {:.2} MB", snapshot.system_memory.used_memory as f64 / 1024.0 / 1024.0);
}
```

---

## 📊 Estatísticas e Relatórios

### Estatísticas da Análise

```rust
// Calcular estatísticas
let start_time = std::time::Instant::now();
let analysis = analyzer.analyze_transaction(tx_hash).await?;
let analysis_time = start_time.elapsed().as_millis() as u64;

let stats = analysis.calculate_stats(analysis_time);

println!("📊 Estatísticas da Análise:");
println!("├─ Total de chamadas: {}", stats.total_calls);
println!("├─ Chamadas falhadas: {}", stats.failed_calls);
println!("├─ Profundidade máxima: {}", stats.max_depth);
println!("├─ Transferências de token: {}", stats.token_transfers);
println!("├─ Contratos criados: {}", stats.contract_creations);
println!("├─ Endereços únicos: {}", stats.unique_addresses);
println!("├─ Gas total usado: {}", stats.total_gas_used);
└─ Tempo de análise: {} ms", stats.analysis_time_ms);
```

### Resumo Textual

```rust
use ethernity_deeptrace::utils::DisplayUtils;

// Criar resumo da análise
let summary = DisplayUtils::create_analysis_summary(&analysis);
println!("{}", summary);
```

### Exemplo de Saída
```
Transação: 0x1234...5678
Bloco: 18500000
Status: Sucesso
Gas usado: 1.2M
Transferências de token: 5
Contratos criados: 1
Padrões detectados: 2
Profundidade máxima: 8

Padrões detectados:
- Token swap detectado (confiança: 0.95)
- Atividade MEV detectada (confiança: 0.87)
```

---

## 🚀 Exemplos Avançados

### Análise Comparativa

```rust
struct ComparativeAnalyzer {
    analyzer: Arc<DeepTraceAnalyzer>,
}

impl ComparativeAnalyzer {
    pub async fn compare_transactions(
        &self,
        tx_hashes: Vec<TransactionHash>
    ) -> Result<ComparisonReport, Error> {
        let mut analyses = Vec::new();
        
        for tx_hash in tx_hashes {
            let analysis = self.analyzer.analyze_transaction(tx_hash).await?;
            analyses.push(analysis);
        }
        
        // Comparar métricas
        let mut report = ComparisonReport::new();
        
        for analysis in &analyses {
            report.add_analysis(&analysis);
        }
        
        // Identificar outliers
        report.identify_outliers();
        
        Ok(report)
    }
}

struct ComparisonReport {
    total_transactions: usize,
    avg_gas_used: U256,
    avg_transfers: f64,
    avg_patterns: f64,
    outliers: Vec<OutlierTransaction>,
}

impl ComparisonReport {
    fn new() -> Self {
        Self {
            total_transactions: 0,
            avg_gas_used: U256::zero(),
            avg_transfers: 0.0,
            avg_patterns: 0.0,
            outliers: Vec::new(),
        }
    }
    
    fn add_analysis(&mut self, analysis: &TransactionAnalysis) {
        self.total_transactions += 1;
        self.avg_gas_used += analysis.gas_used;
        self.avg_transfers += analysis.token_transfers.len() as f64;
        self.avg_patterns += analysis.detected_patterns.len() as f64;
    }
    
    fn identify_outliers(&mut self) {
        // Calcular médias finais
        if self.total_transactions > 0 {
            self.avg_gas_used /= U256::from(self.total_transactions);
            self.avg_transfers /= self.total_transactions as f64;
            self.avg_patterns /= self.total_transactions as f64;
        }
        
        // Lógica para identificar outliers...
    }
}

struct OutlierTransaction {
    tx_hash: TransactionHash,
    reason: OutlierReason,
    metric_value: f64,
    avg_value: f64,
}

enum OutlierReason {
    ExcessiveGasUsage,
    UnusualTransferCount,
    HighPatternCount,
}
```

---

## 🧪 Testes

### Executar Testes
```bash
cd crates/ethernity-deeptrace
cargo test
```

### Testes de Integração
```bash
# Executar com node Ethereum local
cargo test --test integration_tests -- --ignored
```

### Exemplo de Teste
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_trace_analysis() {
        let mock_rpc = MockRpcProvider::new();
        let config = TraceAnalysisConfig::default();
        let analyzer = DeepTraceAnalyzer::new(Arc::new(mock_rpc), Some(config));
        
        let tx_hash = TransactionHash::from_str("0x123...").unwrap();
        let analysis = analyzer.analyze_transaction(tx_hash).await;
        
        assert!(analysis.is_ok());
        let analysis = analysis.unwrap();
        assert!(analysis.call_tree.total_calls() > 0);
    }
    
    #[test]
    fn test_pattern_detection() {
        // Teste de detecção de padrões...
    }
}
```

## 📚 Recursos Adicionais

- [Ethereum Call Traces](https://geth.ethereum.org/docs/developers/evm-tracing)
- [MEV Explained](https://ethereum.org/en/developers/docs/mev/)
- [Flash Loan Attacks](https://consensys.net/diligence/blog/2019/09/stop-using-soliditys-transfer-now/)
- [Reentrancy Attacks](https://consensys.github.io/smart-contract-best-practices/attacks/reentrancy/)
