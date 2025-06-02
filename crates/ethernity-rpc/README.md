# ethernity-rpc

**Cliente RPC otimizado para comunicação com nodes Ethereum**

## Visão Geral

O `ethernity-rpc` fornece um cliente RPC robusto e otimizado para interação com nodes Ethereum. Suporta conexões HTTP e WebSocket, inclui recursos avançados como cache inteligente, pool de conexões, balanceamento de carga e retry automático.

## Características Principais

- ✅ **Múltiplos Transportes**: HTTP e WebSocket
- ✅ **Pool de Conexões**: Balanceamento de carga automático
- ✅ **Cache Inteligente**: Cache configurável com TTL
- ✅ **Retry Automático**: Recuperação automática de falhas
- ✅ **Async/Await**: API totalmente assíncrona
- ✅ **Type Safety**: Integração completa com ethernity-core
- ✅ **Configurável**: Timeouts, limites e comportamentos personalizáveis

## Estrutura do Projeto

```
ethernity-rpc/
├── src/
│   └── lib.rs          # Cliente RPC e todas as implementações
├── Cargo.toml          # Dependências e metadados
└── README.md
```

## Dependências Principais

- **ethernity-core**: Tipos e traits compartilhadas
- **ethers**: Biblioteca Ethereum para Rust
- **web3**: Cliente Web3 para Ethereum
- **reqwest**: Cliente HTTP assíncrono
- **tokio-tungstenite**: Cliente WebSocket
- **lru**: Cache LRU eficiente
- **dashmap**: HashMap concorrente
- **parking_lot**: Primitivas de sincronização

---

## 🔧 Configuração

### RpcConfig - Configuração Principal

```rust
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// URL do endpoint RPC
    pub endpoint: String,
    
    /// Timeout para requisições
    pub timeout: Duration,
    
    /// Número máximo de tentativas
    pub max_retries: u32,
    
    /// Delay entre tentativas
    pub retry_delay: Duration,
    
    /// Habilitar cache
    pub use_cache: bool,
    
    /// TTL do cache
    pub cache_ttl: Duration,
    
    /// Tamanho do pool de conexões
    pub connection_pool_size: usize,
}
```

### Configurações Padrão
```rust
impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8545".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_millis(500),
            use_cache: true,
            cache_ttl: Duration::from_secs(60),
            connection_pool_size: 10,
        }
    }
}
```

### Configurações Personalizadas
```rust
use ethernity_rpc::*;
use std::time::Duration;

// Configuração para produção
let prod_config = RpcConfig {
    endpoint: "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string(),
    timeout: Duration::from_secs(60),
    max_retries: 5,
    retry_delay: Duration::from_millis(1000),
    use_cache: true,
    cache_ttl: Duration::from_secs(300), // 5 minutos
    connection_pool_size: 20,
};

// Configuração para desenvolvimento
let dev_config = RpcConfig {
    endpoint: "http://localhost:8545".to_string(),
    timeout: Duration::from_secs(10),
    max_retries: 1,
    retry_delay: Duration::from_millis(100),
    use_cache: false, // Sem cache para desenvolvimento
    cache_ttl: Duration::from_secs(10),
    connection_pool_size: 2,
};

// Configuração para WebSocket
let ws_config = RpcConfig {
    endpoint: "wss://eth-mainnet.ws.alchemy.com/v2/your-key".to_string(),
    timeout: Duration::from_secs(30),
    // ... outras configurações
    ..Default::default()
};
```

---

## 🌐 Cliente Principal

### EthernityRpcClient

O cliente principal que gerencia conexões e requisições.

#### Criação de Clientes

```rust
use ethernity_rpc::*;

// Cliente HTTP
let http_client = EthernityRpcClient::new_http(config.clone()).await?;

// Cliente WebSocket  
let ws_client = EthernityRpcClient::new_websocket(config.clone()).await?;

// Cliente automático (detecta tipo pela URL)
let auto_client = EthernityRpcClient::new(config).await?;
```

#### Verificação de Conectividade

```rust
// O cliente verifica automaticamente a conectividade na criação
match EthernityRpcClient::new(config).await {
    Ok(client) => {
        println!("✅ Conectado com sucesso ao node Ethereum");
        let block_number = client.get_block_number().await?;
        println!("📦 Bloco atual: {}", block_number);
    },
    Err(e) => {
        eprintln!("❌ Falha na conexão: {}", e);
    }
}
```

### APIs Principais

#### Operações Básicas

```rust
// Obter número do bloco atual
let block_number = client.get_block_number().await?;
println!("Bloco atual: {}", block_number);

// Obter código de contrato
let address = Address::from_str("0x...")?;
let code = client.get_code(address).await?;

if code.is_empty() {
    println!("Endereço é uma EOA (conta externa)");
} else {
    println!("Endereço é um contrato com {} bytes de código", code.len());
}

// Chamada de contrato (call)
let call_data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf(address)
call_data.extend_from_slice(&[0; 32]); // endereço zero-padded

let result = client.call(address, call_data).await?;
let balance = U256::from_big_endian(&result[0..32]);
println!("Saldo: {}", balance);
```

#### Operações de Trace

```rust
use ethernity_core::types::TransactionHash;

// Obter trace de transação
let tx_hash = TransactionHash::from_str("0x123...")?;
let trace_bytes = client.get_transaction_trace(tx_hash).await?;

// Deserializar trace
let trace: serde_json::Value = serde_json::from_slice(&trace_bytes)?;
println!("Trace: {}", serde_json::to_string_pretty(&trace)?);

// Obter recibo de transação
let receipt_bytes = client.get_transaction_receipt(tx_hash).await?;
let receipt: serde_json::Value = serde_json::from_slice(&receipt_bytes)?;

// Verificar status da transação
if let Some(status) = receipt.get("status").and_then(|s| s.as_str()) {
    match status {
        "0x1" => println!("✅ Transação bem-sucedida"),
        "0x0" => println!("❌ Transação falhou"),
        _ => println!("❓ Status desconhecido: {}", status),
    }
}
```

#### Operações de Bloco

```rust
// Obter informações de bloco
let block_bytes = client.get_block(12345678).await?;
let block: serde_json::Value = serde_json::from_slice(&block_bytes)?;

// Extrair informações do bloco
if let Some(block_obj) = block.as_object() {
    println!("Número: {}", block_obj.get("number").unwrap_or(&serde_json::Value::Null));
    println!("Hash: {}", block_obj.get("hash").unwrap_or(&serde_json::Value::Null));
    println!("Timestamp: {}", block_obj.get("timestamp").unwrap_or(&serde_json::Value::Null));
    
    if let Some(transactions) = block_obj.get("transactions").and_then(|t| t.as_array()) {
        println!("Transações: {}", transactions.len());
    }
}
```

### Gerenciamento de Cache

```rust
// Limpar cache
client.clear_cache();

// Obter estatísticas do cache
let stats = client.cache_stats();
println!("Entradas no cache: {}", stats.total_entries);
println!("Entradas expiradas: {}", stats.expired_entries);
println!("Taxa de hit: {:.2}%", stats.cache_hit_ratio * 100.0);

// Exemplo com cache funcionando
let tx_hash = TransactionHash::from_str("0x123...")?;

// Primeira chamada - vai buscar no node
let start = std::time::Instant::now();
let trace1 = client.get_transaction_trace(tx_hash).await?;
println!("Primeira chamada: {:?}", start.elapsed());

// Segunda chamada - vem do cache
let start = std::time::Instant::now();
let trace2 = client.get_transaction_trace(tx_hash).await?;
println!("Segunda chamada (cache): {:?}", start.elapsed());

assert_eq!(trace1, trace2); // Mesmos dados
```

---

## 🏊 Pool de Conexões

### RpcConnectionPool

Gerencia múltiplas conexões para distribuir carga.

```rust
use ethernity_rpc::*;

// Criar pool com 5 conexões
let pool = RpcConnectionPool::new(config, 5).await?;

// Obter cliente do pool (round-robin)
let client1 = pool.get_client();
let client2 = pool.get_client();
let client3 = pool.get_client();

// As requisições são distribuídas automaticamente
let block1 = client1.get_block_number().await?;
let block2 = client2.get_block_number().await?;
let block3 = client3.get_block_number().await?;

// Estatísticas do pool
let stats = pool.pool_stats();
println!("Clientes no pool: {}/{}", stats.active_clients, stats.total_clients);
```

### LoadBalancedRpcClient

Cliente que gerencia automaticamente o balanceamento de carga.

```rust
// Cliente com balanceamento automático
let balanced_client = LoadBalancedRpcClient::new(config).await?;

// Usar como qualquer cliente normal
let trace = balanced_client.get_transaction_trace(tx_hash).await?;
let receipt = balanced_client.get_transaction_receipt(tx_hash).await?;
let block = balanced_client.get_block_number().await?;

// As requisições são automaticamente distribuídas entre as conexões do pool
```

### Exemplo de Uso Intensivo

```rust
use tokio::time::{Duration, Instant};
use futures::future::join_all;

async fn stress_test_pool() -> Result<(), Box<dyn std::error::Error>> {
    let config = RpcConfig {
        endpoint: "https://eth-mainnet.g.alchemy.com/v2/your-key".to_string(),
        connection_pool_size: 10,
        ..Default::default()
    };
    
    let client = LoadBalancedRpcClient::new(config).await?;
    
    // Fazer 100 requisições paralelas
    let start = Instant::now();
    let mut futures = Vec::new();
    
    for _ in 0..100 {
        let client_clone = &client;
        futures.push(async move {
            client_clone.get_block_number().await
        });
    }
    
    let results = join_all(futures).await;
    let duration = start.elapsed();
    
    let successful = results.iter().filter(|r| r.is_ok()).count();
    println!("✅ {}/100 requisições bem-sucedidas em {:?}", successful, duration);
    println!("📊 Taxa: {:.2} req/s", 100.0 / duration.as_secs_f64());
    
    Ok(())
}
```

---

## 🔌 Integração com ethernity-core

### Implementação da Trait RpcProvider

O cliente implementa automaticamente a trait `RpcProvider` do ethernity-core:

```rust
use ethernity_core::traits::RpcProvider;

// Função genérica que aceita qualquer RpcProvider
async fn analyze_contract<T: RpcProvider>(
    provider: &T,
    address: Address
) -> Result<ContractInfo> {
    // Verificar se é contrato
    let code = provider.get_code(address).await?;
    let is_contract = !code.is_empty();
    
    // Obter bloco atual
    let current_block = provider.get_block_number().await?;
    
    // Fazer uma chamada de teste
    let test_call = provider.call(address, vec![]).await;
    let is_callable = test_call.is_ok();
    
    Ok(ContractInfo {
        address,
        is_contract,
        code_size: code.len(),
        current_block,
        is_callable,
    })
}

// Usar com qualquer cliente
let contract_info = analyze_contract(&client, address).await?;
let contract_info_pooled = analyze_contract(&balanced_client, address).await?;
```

### Exemplo de Factory Pattern

```rust
use ethernity_core::traits::RpcProvider;
use std::sync::Arc;

// Factory para criar diferentes tipos de clientes
pub enum RpcClientType {
    Simple,
    Pooled,
    Balanced,
}

pub async fn create_rpc_client(
    config: RpcConfig,
    client_type: RpcClientType
) -> Result<Arc<dyn RpcProvider>, Error> {
    match client_type {
        RpcClientType::Simple => {
            let client = EthernityRpcClient::new(config).await?;
            Ok(Arc::new(client))
        },
        RpcClientType::Pooled => {
            let pool = RpcConnectionPool::new(config, 5).await?;
            let client = pool.get_client();
            Ok(client)
        },
        RpcClientType::Balanced => {
            let client = LoadBalancedRpcClient::new(config).await?;
            Ok(Arc::new(client))
        }
    }
}

// Uso
let client = create_rpc_client(config, RpcClientType::Balanced).await?;
let block_number = client.get_block_number().await?;
```

---

## 📊 Monitoramento e Estatísticas

### Estatísticas de Cache

```rust
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,      // Total de entradas no cache
    pub expired_entries: usize,    // Entradas expiradas
    pub cache_hit_ratio: f64,      // Taxa de acerto (0.0 a 1.0)
}

// Monitorar cache
let stats = client.cache_stats();
println!("📊 Estatísticas do Cache:");
println!("  Entradas ativas: {}", stats.total_entries - stats.expired_entries);
println!("  Entradas expiradas: {}", stats.expired_entries);
println!("  Taxa de acerto: {:.1}%", stats.cache_hit_ratio * 100.0);

// Alertar se cache está com baixa eficiência
if stats.cache_hit_ratio < 0.5 && stats.total_entries > 10 {
    println!("⚠️ Cache com baixa eficiência - considere ajustar TTL");
}
```

### Estatísticas de Pool

```rust
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_clients: usize,    // Total de clientes no pool
    pub active_clients: usize,   // Clientes ativos
}

// Monitorar pool
let stats = pool.pool_stats();
println!("🏊 Estatísticas do Pool:");
println!("  Clientes ativos: {}/{}", stats.active_clients, stats.total_clients);

if stats.active_clients < stats.total_clients {
    println!("⚠️ Alguns clientes podem estar inativos");
}
```

### Monitoramento Avançado

```rust
use tokio::time::{interval, Duration};

async fn monitor_client_health(client: Arc<dyn RpcProvider>) {
    let mut interval = interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        let start = Instant::now();
        match client.get_block_number().await {
            Ok(block) => {
                let latency = start.elapsed();
                println!("✅ Saúde OK - Bloco: {}, Latência: {:?}", block, latency);
                
                if latency > Duration::from_secs(5) {
                    println!("⚠️ Alta latência detectada");
                }
            },
            Err(e) => {
                println!("❌ Cliente não responsivo: {}", e);
            }
        }
    }
}

// Iniciar monitoramento em background
tokio::spawn(monitor_client_health(client.clone()));
```

---

## 🚀 Exemplos Avançados

### Cliente Multi-Network

```rust
use std::collections::HashMap;

struct MultiNetworkClient {
    clients: HashMap<String, Arc<dyn RpcProvider>>,
}

impl MultiNetworkClient {
    pub async fn new() -> Result<Self, Error> {
        let mut clients = HashMap::new();
        
        // Mainnet
        let mainnet_config = RpcConfig {
            endpoint: "https://eth-mainnet.g.alchemy.com/v2/key".to_string(),
            ..Default::default()
        };
        let mainnet_client = LoadBalancedRpcClient::new(mainnet_config).await?;
        clients.insert("mainnet".to_string(), Arc::new(mainnet_client));
        
        // Polygon
        let polygon_config = RpcConfig {
            endpoint: "https://polygon-mainnet.g.alchemy.com/v2/key".to_string(),
            ..Default::default()
        };
        let polygon_client = LoadBalancedRpcClient::new(polygon_config).await?;
        clients.insert("polygon".to_string(), Arc::new(polygon_client));
        
        // BSC
        let bsc_config = RpcConfig {
            endpoint: "https://bsc-dataseed.binance.org/".to_string(),
            ..Default::default()
        };
        let bsc_client = LoadBalancedRpcClient::new(bsc_config).await?;
        clients.insert("bsc".to_string(), Arc::new(bsc_client));
        
        Ok(Self { clients })
    }
    
    pub fn get_client(&self, network: &str) -> Option<&Arc<dyn RpcProvider>> {
        self.clients.get(network)
    }
    
    pub async fn get_all_block_numbers(&self) -> HashMap<String, Result<u64, Error>> {
        let mut results = HashMap::new();
        
        for (network, client) in &self.clients {
            let block_result = client.get_block_number().await;
            results.insert(network.clone(), block_result);
        }
        
        results
    }
}

// Uso
let multi_client = MultiNetworkClient::new().await?;
let block_numbers = multi_client.get_all_block_numbers().await;

for (network, result) in block_numbers {
    match result {
        Ok(block) => println!("{}: Bloco {}", network, block),
        Err(e) => println!("{}: Erro - {}", network, e),
    }
}
```

### Cliente com Retry Personalizado

```rust
use tokio::time::{sleep, Duration};

struct RetryableClient {
    inner: Arc<dyn RpcProvider>,
    max_retries: u32,
    base_delay: Duration,
}

impl RetryableClient {
    pub fn new(client: Arc<dyn RpcProvider>, max_retries: u32) -> Self {
        Self {
            inner: client,
            max_retries,
            base_delay: Duration::from_millis(500),
        }
    }
    
    async fn retry<T, F, Fut>(&self, operation: F) -> Result<T, Error>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, Error>>,
    {
        let mut attempt = 0;
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempt += 1;
                    
                    if attempt > self.max_retries {
                        return Err(e);
                    }
                    
                    // Exponential backoff
                    let delay = self.base_delay * 2_u32.pow(attempt - 1);
                    println!("Tentativa {} falhou, tentando novamente em {:?}", attempt, delay);
                    sleep(delay).await;
                }
            }
        }
    }
}

#[async_trait]
impl RpcProvider for RetryableClient {
    async fn get_block_number(&self) -> Result<u64, Error> {
        self.retry(|| self.inner.get_block_number()).await
    }
    
    // ... outras implementações com retry
}
```

### Cliente com Métricas

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct ClientMetrics {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    total_latency_ms: AtomicU64,
}

impl ClientMetrics {
    pub fn record_request(&self, latency: Duration, success: bool) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms.fetch_add(latency.as_millis() as u64, Ordering::Relaxed);
        
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    pub fn get_stats(&self) -> (u64, u64, u64, f64) {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let failed = self.failed_requests.load(Ordering::Relaxed);
        let total_latency = self.total_latency_ms.load(Ordering::Relaxed);
        
        let avg_latency = if total > 0 {
            total_latency as f64 / total as f64
        } else {
            0.0
        };
        
        (total, successful, failed, avg_latency)
    }
}

struct MetricsClient {
    inner: Arc<dyn RpcProvider>,
    metrics: Arc<ClientMetrics>,
}

impl MetricsClient {
    pub fn new(client: Arc<dyn RpcProvider>) -> Self {
        Self {
            inner: client,
            metrics: Arc::new(ClientMetrics::default()),
        }
    }
    
    pub fn get_metrics(&self) -> Arc<ClientMetrics> {
        self.metrics.clone()
    }
}

#[async_trait]
impl RpcProvider for MetricsClient {
    async fn get_block_number(&self) -> Result<u64, Error> {
        let start = Instant::now();
        let result = self.inner.get_block_number().await;
        let latency = start.elapsed();
        
        self.metrics.record_request(latency, result.is_ok());
        result
    }
    
    // ... outras implementações com métricas
}

// Uso
let base_client = Arc::new(EthernityRpcClient::new(config).await?);
let metrics_client = MetricsClient::new(base_client);
let metrics = metrics_client.get_metrics();

// Fazer algumas requisições
for _ in 0..10 {
    let _ = metrics_client.get_block_number().await;
}

// Ver estatísticas
let (total, successful, failed, avg_latency) = metrics.get_stats();
println!("📊 Métricas:");
println!("  Total: {}", total);
println!("  Sucessos: {} ({:.1}%)", successful, successful as f64 / total as f64 * 100.0);
println!("  Falhas: {} ({:.1}%)", failed, failed as f64 / total as f64 * 100.0);
println!("  Latência média: {:.2}ms", avg_latency);
```

---

## 🧪 Testes

### Executar Testes
```bash
cd crates/ethernity-rpc
cargo test
```

### Testes de Integração
```bash
# Configurar endpoint nos testes
export ETH_RPC_URL="http://your-node:8545"
cargo test --test integration_tests
```

### Exemplo de Teste
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[tokio::test]
    async fn test_client_creation() {
        let config = RpcConfig {
            endpoint: "http://localhost:8545".to_string(),
            timeout: Duration::from_secs(5),
            max_retries: 1,
            ..Default::default()
        };
        
        // Teste pode falhar se não houver node local
        if let Ok(client) = EthernityRpcClient::new(config).await {
            assert!(client.get_block_number().await.is_ok());
        }
    }
    
    #[test]
    fn test_config_defaults() {
        let config = RpcConfig::default();
        assert_eq!(config.endpoint, "http://localhost:8545");
        assert_eq!(config.max_retries, 3);
        assert!(config.use_cache);
    }
}
```

## 📚 Recursos Adicionais

- [JSON-RPC API do Ethereum](https://ethereum.org/en/developers/docs/apis/json-rpc/)
- [Debug Trace API](https://geth.ethereum.org/docs/developers/evm-tracing)
- [Documentação do web3.rs](https://docs.rs/web3/)
- [Documentação do ethers.rs](https://docs.rs/ethers/)
