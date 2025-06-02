# ethernity-sdk

**SDKs para consumidores de eventos Ethernity**

## Visão Geral

O `ethernity-sdk` fornece uma interface simples e poderosa para consumir eventos blockchain processados pela plataforma Ethernity. Através de integração com Apache Kafka, permite que aplicações se inscrevam em diferentes tipos de eventos e processem-nos de forma assíncrona e escalável.

## Características Principais

- 🎯 **Subscrições Simples**: Interface intuitiva para subscrever eventos específicos
- 🔧 **Filtros Personalizáveis**: Sistema flexível de filtros para eventos
- ⚡ **Handlers Assíncronos**: Processamento não-bloqueante de eventos
- 🔐 **Autenticação Segura**: Suporte a SASL/SSL para Kafka
- 📊 **Grupos de Consumidores**: Balanceamento automático de carga
- 🔄 **Reconexão Automática**: Recuperação automática de falhas de conexão
- 📈 **Escalabilidade**: Suporte a processamento paralelo
- 🛡️ **Tratamento de Erros**: Gerenciamento robusto de falhas

## Estrutura do Projeto

```
ethernity-sdk/
├── src/
│   └── lib.rs          # Interface principal do SDK
├── Cargo.toml          # Dependências e metadados
└── README.md
```

## Dependências Principais

- **ethernity-core**: Tipos e traits compartilhadas
- **rdkafka**: Cliente Kafka para Rust
- **schema_registry_converter**: Integração com Schema Registry
- **tokio**: Runtime assíncrono
- **serde**: Serialização e deserialização
- **futures**: Utilities para programação assíncrona
- **uuid**: Geração de identificadores únicos
- **avro-rs**: Serialização Avro

---

## ⚙️ Configuração

### ConsumerConfig

```rust
#[derive(Debug, Clone)]
pub struct ConsumerConfig {
    /// Lista de brokers Kafka
    pub kafka_brokers: Vec<String>,
    
    /// ID do grupo de consumidores
    pub consumer_group: String,
    
    /// Chave de API para autenticação (opcional)
    pub api_key: Option<String>,
    
    /// Segredo de API para autenticação (opcional)
    pub api_secret: Option<String>,
    
    /// Habilitar commit automático
    pub auto_commit: bool,
    
    /// Máximo de registros por poll
    pub max_poll_records: i32,
    
    /// Timeout da sessão em milissegundos
    pub session_timeout_ms: i32,
}
```

### Builder Pattern para Configuração

```rust
use ethernity_sdk::*;

// Configuração básica local
let config = ConsumerConfig::builder()
    .kafka_brokers("localhost:9092")
    .consumer_group("my-application")
    .auto_commit(true)
    .build()?;

// Configuração para produção com autenticação
let prod_config = ConsumerConfig::builder()
    .kafka_brokers("kafka-cluster.company.com:9092")
    .consumer_group("production-monitor")
    .api_key("your-api-key")
    .api_secret("your-api-secret")
    .auto_commit(false) // Commit manual para garantias de entrega
    .max_poll_records(1000)
    .session_timeout_ms(30000)
    .build()?;

// Configuração para desenvolvimento
let dev_config = ConsumerConfig::builder()
    .kafka_brokers("dev-kafka:9092")
    .consumer_group("dev-team")
    .auto_commit(true)
    .max_poll_records(100)
    .session_timeout_ms(10000)
    .build()?;

// Configuração para múltiplos brokers
let cluster_config = ConsumerConfig::builder()
    .kafka_brokers("broker1:9092,broker2:9092,broker3:9092")
    .consumer_group("cluster-consumer")
    .auto_commit(false)
    .build()?;
```

### Configurações de Segurança

```rust
// Configuração com SSL/SASL
let secure_config = ConsumerConfig::builder()
    .kafka_brokers("secure-kafka.company.com:9093")
    .consumer_group("secure-consumer")
    .api_key("username")
    .api_secret("password")
    .auto_commit(false)
    .build()?;

// Configuração para Confluent Cloud
let confluent_config = ConsumerConfig::builder()
    .kafka_brokers("pkc-xxxxx.us-west-2.aws.confluent.cloud:9092")
    .consumer_group("confluent-consumer")
    .api_key("confluent-api-key")
    .api_secret("confluent-api-secret")
    .auto_commit(true)
    .build()?;
```

---

## 📡 Consumidor Principal

### EthernityConsumer

O componente central que gerencia subscrições e processamento de eventos.

```rust
use ethernity_sdk::*;
use ethernity_core::types::*;

// Criar o consumidor
let consumer = EthernityConsumer::new(config).await?;

// O consumidor está pronto para receber subscrições
println!("✅ Consumidor criado com sucesso");
```

### Ciclo de Vida do Consumidor

```rust
// 1. Criar consumidor
let consumer = EthernityConsumer::new(config).await?;

// 2. Configurar subscrições (antes de iniciar)
consumer.subscribe(EventType::TokenSwap)
    .with_handler(|event| async move {
        println!("Token swap: {:?}", event);
    })
    .start()
    .await?;

consumer.subscribe(EventType::LargeTransfer)
    .with_handler(|event| async move {
        println!("Large transfer: {:?}", event);
    })
    .start()
    .await?;

// 3. Iniciar processamento
consumer.start().await?;
println!("🚀 Consumidor iniciado");

// 4. O consumidor processa eventos em background
// Sua aplicação pode fazer outras tarefas aqui
tokio::time::sleep(Duration::from_secs(300)).await;

// 5. Parar graciosamente
consumer.stop().await?;
println!("⏹️ Consumidor parado");
```

---

## 🎯 Sistema de Subscrições

### Subscrições Básicas

```rust
use ethernity_core::types::EventType;

// Subscrição simples para swaps de tokens
consumer.subscribe(EventType::TokenSwap)
    .with_handler(|event| async move {
        println!("🔄 Token swap detectado!");
        
        // Extrair informações do evento
        if let Some(token_in) = event.get("token_in") {
            println!("  Token entrada: {}", token_in);
        }
        
        if let Some(token_out) = event.get("token_out") {
            println!("  Token saída: {}", token_out);
        }
        
        if let Some(amount) = event.get("amount") {
            println!("  Quantidade: {}", amount);
        }
        
        if let Some(dex) = event.get("dex_protocol") {
            println!("  DEX: {}", dex);
        }
    })
    .start()
    .await?;

// Subscrição para criação de tokens ERC20
consumer.subscribe(EventType::Erc20Created)
    .with_handler(|event| async move {
        println!("🪙 Novo token ERC20 criado!");
        
        if let Some(address) = event.get("contract_address") {
            println!("  Endereço: {}", address);
        }
        
        if let Some(name) = event.get("token_name") {
            println!("  Nome: {}", name);
        }
        
        if let Some(symbol) = event.get("token_symbol") {
            println!("  Símbolo: {}", symbol);
        }
        
        if let Some(creator) = event.get("creator") {
            println!("  Criador: {}", creator);
        }
    })
    .start()
    .await?;
```

### Subscrições com Filtros

```rust
// Filtrar apenas transferências grandes (> $1M)
consumer.subscribe(EventType::LargeTransfer)
    .with_filter(|event| {
        event.get("usd_value")
            .and_then(|v| v.as_f64())
            .map(|amount| amount > 1_000_000.0)
            .unwrap_or(false)
    })
    .with_handler(|event| async move {
        println!("💰 Transferência milionária detectada!");
        
        if let Some(amount) = event.get("usd_value") {
            println!("  Valor: ${:.2}", amount.as_f64().unwrap_or(0.0));
        }
        
        if let Some(from) = event.get("from_address") {
            println!("  De: {}", from);
        }
        
        if let Some(to) = event.get("to_address") {
            println!("  Para: {}", to);
        }
    })
    .start()
    .await?;

// Filtrar apenas tokens USDC
consumer.subscribe(EventType::TokenSwap)
    .with_filter(|event| {
        let token_in = event.get("token_in").and_then(|v| v.as_str()).unwrap_or("");
        let token_out = event.get("token_out").and_then(|v| v.as_str()).unwrap_or("");
        
        token_in.contains("USDC") || token_out.contains("USDC")
    })
    .with_handler(|event| async move {
        println!("🪙 Swap envolvendo USDC!");
        // Processar swap específico
    })
    .start()
    .await?;

// Filtrar por DEX específico
consumer.subscribe(EventType::TokenSwap)
    .with_filter(|event| {
        event.get("dex_protocol")
            .and_then(|v| v.as_str())
            .map(|dex| dex == "UniswapV3")
            .unwrap_or(false)
    })
    .with_handler(|event| async move {
        println!("🦄 Swap no Uniswap V3!");
        // Processar swap específico do Uniswap
    })
    .start()
    .await?;
```

### Filtros Avançados

```rust
// Filtro composto para liquidações grandes em protocolos específicos
consumer.subscribe(EventType::Liquidation)
    .with_filter(|event| {
        // Verificar se é uma liquidação grande
        let is_large = event.get("liquidated_amount_usd")
            .and_then(|v| v.as_f64())
            .map(|amount| amount > 500_000.0)
            .unwrap_or(false);
        
        // Verificar se é em um protocolo de interesse
        let is_target_protocol = event.get("protocol")
            .and_then(|v| v.as_str())
            .map(|protocol| {
                matches!(protocol, "Aave" | "Compound" | "MakerDAO")
            })
            .unwrap_or(false);
        
        is_large && is_target_protocol
    })
    .with_handler(|event| async move {
        println!("⚡ Liquidação significativa em protocolo principal!");
        
        // Processar liquidação crítica
        process_critical_liquidation(&event).await;
    })
    .start()
    .await?;

// Filtro para MEV com base na lucratividade
consumer.subscribe(EventType::MevActivity)
    .with_filter(|event| {
        // Filtrar apenas MEV altamente lucrativo
        let profit = event.get("estimated_profit_usd")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        
        let mev_type = event.get("mev_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        // Arbitragem > $10k OU sandwich attack > $5k
        (mev_type == "arbitrage" && profit > 10_000.0) ||
        (mev_type == "sandwich" && profit > 5_000.0)
    })
    .with_handler(|event| async move {
        println!("🤖 MEV altamente lucrativo detectado!");
        
        // Analisar estratégia MEV
        analyze_mev_strategy(&event).await;
    })
    .start()
    .await?;
```

---

## 🔧 Handlers Especializados

### Handler Básico

```rust
// Handler simples para logging
consumer.subscribe(EventType::Erc20Created)
    .with_handler(|event| async move {
        println!("Evento recebido: {:?}", event);
    })
    .start()
    .await?;
```

### Handlers com Processamento Assíncrono

```rust
// Handler que faz chamadas de API
consumer.subscribe(EventType::LargeTransfer)
    .with_handler(|event| async move {
        // Extrair dados do evento
        let from_address = event.get("from_address")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let to_address = event.get("to_address")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let amount_usd = event.get("usd_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        
        // Chamada assíncrona para API externa
        if let Err(e) = send_webhook_notification(from_address, to_address, amount_usd).await {
            eprintln!("Erro ao enviar webhook: {}", e);
        }
        
        // Salvar no banco de dados
        if let Err(e) = save_to_database(&event).await {
            eprintln!("Erro ao salvar no banco: {}", e);
        }
        
        // Enviar para sistema de alertas
        if amount_usd > 10_000_000.0 {
            send_urgent_alert(&event).await;
        }
    })
    .start()
    .await?;

async fn send_webhook_notification(from: &str, to: &str, amount: f64) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "type": "large_transfer",
        "from": from,
        "to": to,
        "amount_usd": amount,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    let response = client
        .post("https://api.company.com/webhooks/crypto-events")
        .json(&payload)
        .send()
        .await?;
    
    if response.status().is_success() {
        println!("✅ Webhook enviado com sucesso");
    } else {
        println!("❌ Falha no webhook: {}", response.status());
    }
    
    Ok(())
}
```

### Handler com Análise Complexa

```rust
// Handler que analisa padrões de trading
consumer.subscribe(EventType::TokenSwap)
    .with_handler(|event| async move {
        // Estrutura para análise de swap
        let swap_analysis = analyze_swap(&event).await;
        
        match swap_analysis {
            SwapAnalysis::Normal { .. } => {
                // Swap normal, apenas log
                log_normal_swap(&event).await;
            },
            SwapAnalysis::Suspicious { reason, confidence } => {
                println!("⚠️ Swap suspeito detectado!");
                println!("   Razão: {:?}", reason);
                println!("   Confiança: {:.2}%", confidence * 100.0);
                
                // Alertar equipe de segurança
                alert_security_team(&event, &reason).await;
            },
            SwapAnalysis::HighVolume { volume_usd } => {
                println!("📈 Swap de alto volume: ${:.2}", volume_usd);
                
                // Alertar equipe de trading
                alert_trading_team(&event).await;
            }
        }
    })
    .start()
    .await?;

#[derive(Debug)]
enum SwapAnalysis {
    Normal { volume_usd: f64 },
    Suspicious { reason: SuspiciousReason, confidence: f64 },
    HighVolume { volume_usd: f64 },
}

#[derive(Debug)]
enum SuspiciousReason {
    PossibleSandwich,
    UnusualSlippage,
    HighFrequency,
    FlashLoanPattern,
}

async fn analyze_swap(event: &serde_json::Value) -> SwapAnalysis {
    let volume_usd = event.get("volume_usd")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    
    let slippage = event.get("slippage_percent")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    
    let trader = event.get("trader_address")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    // Verificar volume alto
    if volume_usd > 1_000_000.0 {
        return SwapAnalysis::HighVolume { volume_usd };
    }
    
    // Verificar slippage anormal
    if slippage > 5.0 {
        return SwapAnalysis::Suspicious {
            reason: SuspiciousReason::UnusualSlippage,
            confidence: 0.7,
        };
    }
    
    // Verificar padrões de alta frequência
    let recent_swaps = count_recent_swaps_by_trader(trader).await;
    if recent_swaps > 10 {
        return SwapAnalysis::Suspicious {
            reason: SuspiciousReason::HighFrequency,
            confidence: 0.8,
        };
    }
    
    SwapAnalysis::Normal { volume_usd }
}
```

### Handler com Estado Persistente

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

// Estado compartilhado entre handlers
#[derive(Default)]
struct TradingState {
    trader_volumes: HashMap<String, f64>,
    token_prices: HashMap<String, f64>,
    suspicious_addresses: HashSet<String>,
}

let trading_state = Arc::new(RwLock::new(TradingState::default()));

// Handler que mantém estado
let state_clone = trading_state.clone();
consumer.subscribe(EventType::TokenSwap)
    .with_handler(move |event| {
        let state = state_clone.clone();
        async move {
            let trader = event.get("trader_address")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            
            let volume = event.get("volume_usd")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            
            // Atualizar estado
            {
                let mut state_guard = state.write().await;
                let total_volume = state_guard.trader_volumes
                    .entry(trader.clone())
                    .or_insert(0.0);
                *total_volume += volume;
                
                // Marcar como suspeito se volume total > $10M
                if *total_volume > 10_000_000.0 {
                    state_guard.suspicious_addresses.insert(trader.clone());
                    println!("🚨 Trader suspeito identificado: {} (volume total: ${:.2})", 
                        trader, *total_volume);
                }
            }
            
            // Verificar se é endereço suspeito
            {
                let state_guard = state.read().await;
                if state_guard.suspicious_addresses.contains(&trader) {
                    println!("⚠️ Atividade de endereço suspeito: {}", trader);
                    monitor_suspicious_activity(&event).await;
                }
            }
        }
    })
    .start()
    .await?;
```

---

## 🎭 Casos de Uso Especializados

### Monitor de Segurança DeFi

```rust
struct DefiSecurityMonitor {
    consumer: EthernityConsumer,
    alert_channel: tokio::sync::mpsc::Sender<SecurityAlert>,
}

impl DefiSecurityMonitor {
    pub async fn new(config: ConsumerConfig) -> Result<Self, Error> {
        let consumer = EthernityConsumer::new(config).await?;
        let (alert_sender, alert_receiver) = tokio::sync::mpsc::channel(1000);
        
        // Configurar subscrições de segurança
        Self::setup_security_subscriptions(&consumer, alert_sender.clone()).await?;
        
        // Iniciar processador de alertas
        tokio::spawn(Self::process_alerts(alert_receiver));
        
        Ok(Self {
            consumer,
            alert_channel: alert_sender,
        })
    }
    
    async fn setup_security_subscriptions(
        consumer: &EthernityConsumer,
        alert_sender: tokio::sync::mpsc::Sender<SecurityAlert>
    ) -> Result<(), Error> {
        // Monitor de rug pulls
        let alert_sender_clone = alert_sender.clone();
        consumer.subscribe(EventType::RugPullWarning)
            .with_filter(|event| {
                event.get("confidence")
                    .and_then(|v| v.as_f64())
                    .map(|conf| conf > 0.7)
                    .unwrap_or(false)
            })
            .with_handler(move |event| {
                let sender = alert_sender_clone.clone();
                async move {
                    let alert = SecurityAlert::RugPull {
                        token_address: event.get("token_address")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        confidence: event.get("confidence")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        estimated_loss: event.get("estimated_loss_usd")
                            .and_then(|v| v.as_f64()),
                    };
                    
                    if let Err(e) = sender.send(alert).await {
                        eprintln!("Erro ao enviar alerta: {}", e);
                    }
                }
            })
            .start()
            .await?;
        
        // Monitor de liquidações
        let alert_sender_clone = alert_sender.clone();
        consumer.subscribe(EventType::Liquidation)
            .with_filter(|event| {
                event.get("liquidated_amount_usd")
                    .and_then(|v| v.as_f64())
                    .map(|amount| amount > 1_000_000.0)
                    .unwrap_or(false)
            })
            .with_handler(move |event| {
                let sender = alert_sender_clone.clone();
                async move {
                    let alert = SecurityAlert::LargeLiquidation {
                        protocol: event.get("protocol")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        amount_usd: event.get("liquidated_amount_usd")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        liquidated_user: event.get("liquidated_user")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    };
                    
                    if let Err(e) = sender.send(alert).await {
                        eprintln!("Erro ao enviar alerta: {}", e);
                    }
                }
            })
            .start()
            .await?;
        
        // Monitor de MEV
        let alert_sender_clone = alert_sender.clone();
        consumer.subscribe(EventType::MevActivity)
            .with_filter(|event| {
                event.get("estimated_profit_usd")
                    .and_then(|v| v.as_f64())
                    .map(|profit| profit > 50_000.0)
                    .unwrap_or(false)
            })
            .with_handler(move |event| {
                let sender = alert_sender_clone.clone();
                async move {
                    let alert = SecurityAlert::HighValueMev {
                        mev_type: event.get("mev_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        profit_usd: event.get("estimated_profit_usd")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        mev_bot: event.get("mev_bot_address")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    };
                    
                    if let Err(e) = sender.send(alert).await {
                        eprintln!("Erro ao enviar alerta: {}", e);
                    }
                }
            })
            .start()
            .await?;
        
        Ok(())
    }
    
    async fn process_alerts(mut receiver: tokio::sync::mpsc::Receiver<SecurityAlert>) {
        while let Some(alert) = receiver.recv().await {
            match alert {
                SecurityAlert::RugPull { token_address, confidence, estimated_loss } => {
                    println!("🚨 RUG PULL DETECTADO!");
                    if let Some(token) = token_address {
                        println!("   Token: {}", token);
                    }
                    println!("   Confiança: {:.1}%", confidence * 100.0);
                    if let Some(loss) = estimated_loss {
                        println!("   Perda estimada: ${:.2}", loss);
                    }
                    
                    // Enviar para sistemas de alerta
                    send_critical_alert(&alert).await;
                    
                    // Adicionar à blacklist
                    if let Some(token) = token_address {
                        add_to_blacklist(&token).await;
                    }
                },
                SecurityAlert::LargeLiquidation { protocol, amount_usd, liquidated_user } => {
                    println!("⚡ LIQUIDAÇÃO GRANDE!");
                    if let Some(prot) = protocol {
                        println!("   Protocolo: {}", prot);
                    }
                    println!("   Valor: ${:.2}", amount_usd);
                    if let Some(user) = liquidated_user {
                        println!("   Usuário: {}", user);
                    }
                    
                    // Analisar impacto no mercado
                    analyze_market_impact(amount_usd).await;
                },
                SecurityAlert::HighValueMev { mev_type, profit_usd, mev_bot } => {
                    println!("🤖 MEV DE ALTO VALOR!");
                    if let Some(mev_type) = mev_type {
                        println!("   Tipo: {}", mev_type);
                    }
                    println!("   Lucro: ${:.2}", profit_usd);
                    if let Some(bot) = mev_bot {
                        println!("   Bot: {}", bot);
                    }
                    
                    // Analisar estratégia MEV
                    analyze_mev_strategy_detailed(&alert).await;
                }
            }
        }
    }
    
    pub async fn start(&self) -> Result<(), Error> {
        self.consumer.start().await
    }
    
    pub async fn stop(&self) -> Result<(), Error> {
        self.consumer.stop().await
    }
}

#[derive(Debug, Clone)]
enum SecurityAlert {
    RugPull {
        token_address: Option<String>,
        confidence: f64,
        estimated_loss: Option<f64>,
    },
    LargeLiquidation {
        protocol: Option<String>,
        amount_usd: f64,
        liquidated_user: Option<String>,
    },
    HighValueMev {
        mev_type: Option<String>,
        profit_usd: f64,
        mev_bot: Option<String>,
    },
}

// Uso do monitor
let security_monitor = DefiSecurityMonitor::new(config).await?;
security_monitor.start().await?;

// Manter rodando
tokio::signal::ctrl_c().await?;
security_monitor.stop().await?;
```

### Sistema de Métricas e Analytics

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

struct DeFiAnalytics {
    consumer: EthernityConsumer,
    metrics: Arc<RwLock<MetricsStore>>,
}

#[derive(Default)]
struct MetricsStore {
    daily_volume: HashMap<String, f64>,      // DEX -> volume
    token_transfers: HashMap<String, u64>,    // token -> count
    mev_profits: Vec<f64>,                   // profits list
    liquidation_volumes: Vec<f64>,           // liquidation amounts
    flash_loan_volumes: Vec<f64>,            // flash loan amounts
}

impl DeFiAnalytics {
    pub async fn new(config: ConsumerConfig) -> Result<Self, Error> {
        let consumer = EthernityConsumer::new(config).await?;
        let metrics = Arc::new(RwLock::new(MetricsStore::default()));
        
        Self::setup_metrics_collection(&consumer, metrics.clone()).await?;
        
        Ok(Self { consumer, metrics })
    }
    
    async fn setup_metrics_collection(
        consumer: &EthernityConsumer,
        metrics: Arc<RwLock<MetricsStore>>
    ) -> Result<(), Error> {
        // Coletar volume de swaps por DEX
        let metrics_clone = metrics.clone();
        consumer.subscribe(EventType::TokenSwap)
            .with_handler(move |event| {
                let metrics = metrics_clone.clone();
                async move {
                    let dex = event.get("dex_protocol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    
                    let volume = event.get("volume_usd")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    
                    let mut metrics_guard = metrics.write().await;
                    *metrics_guard.daily_volume.entry(dex).or_insert(0.0) += volume;
                }
            })
            .start()
            .await?;
        
        // Coletar métricas de transferências
        let metrics_clone = metrics.clone();
        consumer.subscribe(EventType::LargeTransfer)
            .with_handler(move |event| {
                let metrics = metrics_clone.clone();
                async move {
                    let token = event.get("token_symbol")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    
                    let mut metrics_guard = metrics.write().await;
                    *metrics_guard.token_transfers.entry(token).or_insert(0) += 1;
                }
            })
            .start()
            .await?;
        
        // Coletar métricas de MEV
        let metrics_clone = metrics.clone();
        consumer.subscribe(EventType::MevActivity)
            .with_handler(move |event| {
                let metrics = metrics_clone.clone();
                async move {
                    let profit = event.get("estimated_profit_usd")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.mev_profits.push(profit);
                }
            })
            .start()
            .await?;
        
        // Coletar métricas de liquidações
        let metrics_clone = metrics.clone();
        consumer.subscribe(EventType::Liquidation)
            .with_handler(move |event| {
                let metrics = metrics_clone.clone();
                async move {
                    let amount = event.get("liquidated_amount_usd")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.liquidation_volumes.push(amount);
                }
            })
            .start()
            .await?;
        
        // Coletar métricas de flash loans
        let metrics_clone = metrics.clone();
        consumer.subscribe(EventType::FlashLoan)
            .with_handler(move |event| {
                let metrics = metrics_clone.clone();
                async move {
                    let amount = event.get("amount_usd")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    
                    let mut metrics_guard = metrics.write().await;
                    metrics_guard.flash_loan_volumes.push(amount);
                }
            })
            .start()
            .await?;
        
        Ok(())
    }
    
    pub async fn get_daily_report(&self) -> DailyReport {
        let metrics = self.metrics.read().await;
        
        // Calcular estatísticas
        let total_dex_volume: f64 = metrics.daily_volume.values().sum();
        let top_dex = metrics.daily_volume.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(dex, volume)| (dex.clone(), *volume));
        
        let total_transfers: u64 = metrics.token_transfers.values().sum();
        let top_token = metrics.token_transfers.iter()
            .max_by(|a, b| a.1.cmp(b.1))
            .map(|(token, count)| (token.clone(), *count));
        
        let total_mev_profit: f64 = metrics.mev_profits.iter().sum();
        let avg_mev_profit = if !metrics.mev_profits.is_empty() {
            total_mev_profit / metrics.mev_profits.len() as f64
        } else {
            0.0
        };
        
        let total_liquidations: f64 = metrics.liquidation_volumes.iter().sum();
        let total_flash_loans: f64 = metrics.flash_loan_volumes.iter().sum();
        
        DailyReport {
            date: chrono::Utc::now().date_naive(),
            total_dex_volume,
            top_dex,
            total_transfers,
            top_token,
            total_mev_profit,
            avg_mev_profit,
            mev_transactions: metrics.mev_profits.len(),
            total_liquidations,
            liquidation_count: metrics.liquidation_volumes.len(),
            total_flash_loans,
            flash_loan_count: metrics.flash_loan_volumes.len(),
        }
    }
    
    pub async fn start(&self) -> Result<(), Error> {
        self.consumer.start().await
    }
}

#[derive(Debug)]
struct DailyReport {
    date: chrono::NaiveDate,
    total_dex_volume: f64,
    top_dex: Option<(String, f64)>,
    total_transfers: u64,
    top_token: Option<(String, u64)>,
    total_mev_profit: f64,
    avg_mev_profit: f64,
    mev_transactions: usize,
    total_liquidations: f64,
    liquidation_count: usize,
    total_flash_loans: f64,
    flash_loan_count: usize,
}

impl std::fmt::Display for DailyReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "📊 Relatório Diário - {}\n", self.date)?;
        write!(f, "┌─ Volume DEX Total: ${:.2}\n", self.total_dex_volume)?;
        if let Some((dex, volume)) = &self.top_dex {
            write!(f, "├─ Top DEX: {} (${:.2})\n", dex, volume)?;
        }
        write!(f, "├─ Transferências Totais: {}\n", self.total_transfers)?;
        if let Some((token, count)) = &self.top_token {
            write!(f, "├─ Top Token: {} ({} transferências)\n", token, count)?;
        }
        write!(f, "├─ Lucro MEV Total: ${:.2}\n", self.total_mev_profit)?;
        write!(f, "├─ Lucro MEV Médio: ${:.2}\n", self.avg_mev_profit)?;
        write!(f, "├─ Transações MEV: {}\n", self.mev_transactions)?;
        write!(f, "├─ Volume de Liquidações: ${:.2} ({} liquidações)\n", self.total_liquidations, self.liquidation_count)?;
        write!(f, "└─ Volume Flash Loans: ${:.2} ({} operações)\n", self.total_flash_loans, self.flash_loan_count)
    }
}

// Uso do sistema de analytics
let analytics = DeFiAnalytics::new(config).await?;
analytics.start().await?;

// Gerar relatório diário
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60)); // 24 horas
    
    loop {
        interval.tick().await;
        let report = analytics.get_daily_report().await;
        println!("{}", report);
        
        // Salvar relatório
        save_daily_report(&report).await;
    }
});
```

---

## 🔄 Gerenciamento Avançado

### Reconexão Automática

```rust
struct ResilientConsumer {
    config: ConsumerConfig,
    consumer: Option<EthernityConsumer>,
    max_reconnect_attempts: u32,
    reconnect_delay: Duration,
}

impl ResilientConsumer {
    pub fn new(config: ConsumerConfig) -> Self {
        Self {
            config,
            consumer: None,
            max_reconnect_attempts: 10,
            reconnect_delay: Duration::from_secs(5),
        }
    }
    
    pub async fn start_with_resilience(&mut self) -> Result<(), Error> {
        let mut attempts = 0;
        
        loop {
            match EthernityConsumer::new(self.config.clone()).await {
                Ok(consumer) => {
                    println!("✅ Consumidor conectado");
                    
                    // Configurar subscrições
                    self.setup_subscriptions(&consumer).await?;
                    
                    // Iniciar consumo
                    if let Err(e) = consumer.start().await {
                        println!("❌ Erro ao iniciar consumidor: {}", e);
                        
                        attempts += 1;
                        if attempts >= self.max_reconnect_attempts {
                            return Err(Error::Other("Máximo de tentativas de reconexão excedido".to_string()));
                        }
                        
                        println!("🔄 Tentando reconectar em {:?}...", self.reconnect_delay);
                        tokio::time::sleep(self.reconnect_delay).await;
                        continue;
                    }
                    
                    self.consumer = Some(consumer);
                    break;
                },
                Err(e) => {
                    println!("❌ Falha na conexão: {}", e);
                    
                    attempts += 1;
                    if attempts >= self.max_reconnect_attempts {
                        return Err(Error::Other("Máximo de tentativas de conexão excedido".to_string()));
                    }
                    
                    println!("🔄 Tentando reconectar em {:?}... (tentativa {}/{})", 
                        self.reconnect_delay, attempts, self.max_reconnect_attempts);
                    tokio::time::sleep(self.reconnect_delay).await;
                }
            }
        }
        
        Ok(())
    }
    
    async fn setup_subscriptions(&self, consumer: &EthernityConsumer) -> Result<(), Error> {
        // Reconfigurar todas as subscrições
        consumer.subscribe(EventType::TokenSwap)
            .with_handler(|event| async move {
                println!("Swap: {:?}", event);
            })
            .start()
            .await?;
        
        consumer.subscribe(EventType::LargeTransfer)
            .with_handler(|event| async move {
                println!("Transfer: {:?}", event);
            })
            .start()
            .await?;
        
        Ok(())
    }
}
```

### Processamento em Lote

```rust
struct BatchProcessor {
    consumer: EthernityConsumer,
    batch_size: usize,
    flush_interval: Duration,
    event_buffer: Arc<RwLock<Vec<serde_json::Value>>>,
}

impl BatchProcessor {
    pub async fn new(config: ConsumerConfig, batch_size: usize) -> Result<Self, Error> {
        let consumer = EthernityConsumer::new(config).await?;
        let event_buffer = Arc::new(RwLock::new(Vec::with_capacity(batch_size)));
        
        Ok(Self {
            consumer,
            batch_size,
            flush_interval: Duration::from_secs(10),
            event_buffer,
        })
    }
    
    pub async fn start_batch_processing(&self) -> Result<(), Error> {
        // Configurar coleta de eventos
        let buffer_clone = self.event_buffer.clone();
        self.consumer.subscribe(EventType::TokenSwap)
            .with_handler(move |event| {
                let buffer = buffer_clone.clone();
                async move {
                    let mut buffer_guard = buffer.write().await;
                    buffer_guard.push(event);
                }
            })
            .start()
            .await?;
        
        // Iniciar processamento periódico
        let buffer_clone = self.event_buffer.clone();
        let batch_size = self.batch_size;
        let flush_interval = self.flush_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(flush_interval);
            
            loop {
                interval.tick().await;
                
                let batch = {
                    let mut buffer_guard = buffer_clone.write().await;
                    if buffer_guard.len() >= batch_size || !buffer_guard.is_empty() {
                        std::mem::take(&mut *buffer_guard)
                    } else {
                        continue;
                    }
                };
                
                if !batch.is_empty() {
                    println!("📦 Processando lote de {} eventos", batch.len());
                    process_event_batch(batch).await;
                }
            }
        });
        
        // Iniciar consumidor
        self.consumer.start().await
    }
}

async fn process_event_batch(events: Vec<serde_json::Value>) {
    // Processar eventos em lote de forma eficiente
    println!("Processando lote de {} eventos", events.len());
    
    // Agrupar por tipo
    let mut swaps = Vec::new();
    let mut transfers = Vec::new();
    
    for event in events {
        if let Some(event_type) = event.get("event_type").and_then(|v| v.as_str()) {
            match event_type {
                "token_swap" => swaps.push(event),
                "large_transfer" => transfers.push(event),
                _ => {}
            }
        }
    }
    
    // Processar cada tipo em paralelo
    let swap_task = tokio::spawn(async move {
        for swap in swaps {
            process_swap(&swap).await;
        }
    });
    
    let transfer_task = tokio::spawn(async move {
        for transfer in transfers {
            process_transfer(&transfer).await;
        }
    });
    
    // Aguardar conclusão
    let _ = tokio::join!(swap_task, transfer_task);
    
    println!("✅ Lote processado com sucesso");
}
```

---

## 🧪 Testes

### Executar Testes
```bash
cd crates/ethernity-sdk
cargo test
```

### Testes de Unidade
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;
    
    #[test]
    fn test_config_builder() {
        let config = ConsumerConfig::builder()
            .kafka_brokers("localhost:9092")
            .consumer_group("test-group")
            .build()
            .unwrap();
        
        assert_eq!(config.kafka_brokers, vec!["localhost:9092"]);
        assert_eq!(config.consumer_group, "test-group");
    }
    
    #[test]
    fn test_config_validation() {
        // Deve falhar sem brokers
        let result = ConsumerConfig::builder()
            .consumer_group("test-group")
            .build();
        
        assert!(result.is_err());
        
        // Deve falhar sem consumer group
        let result = ConsumerConfig::builder()
            .kafka_brokers("localhost:9092")
            .build();
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_subscription_filter() {
        // Teste de filtros de subscrição
        let filter = |event: &serde_json::Value| {
            event.get("amount")
                .and_then(|v| v.as_f64())
                .map(|amount| amount > 1000.0)
                .unwrap_or(false)
        };
        
        let event1 = serde_json::json!({"amount": 500.0});
        let event2 = serde_json::json!({"amount": 1500.0});
        
        assert!(!filter(&event1));
        assert!(filter(&event2));
    }
}
```

### Testes de Integração
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Requer Kafka rodando
    async fn test_consumer_integration() {
        let config = ConsumerConfig::builder()
            .kafka_brokers("localhost:9092")
            .consumer_group("test-integration")
            .build()
            .unwrap();
        
        let consumer = EthernityConsumer::new(config).await.unwrap();
        
        let received_events = Arc::new(RwLock::new(Vec::new()));
        let events_clone = received_events.clone();
        
        consumer.subscribe(EventType::TokenSwap)
            .with_handler(move |event| {
                let events = events_clone.clone();
                async move {
                    events.write().await.push(event);
                }
            })
            .start()
            .await
            .unwrap();
        
        consumer.start().await.unwrap();
        
        // Aguardar alguns eventos
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        consumer.stop().await.unwrap();
        
        let events = received_events.read().await;
        // Verificar se eventos foram recebidos (depende do ambiente)
        println!("Eventos recebidos: {}", events.len());
    }
}
```

---

## 📚 Recursos Adicionais

- [Apache Kafka Documentation](https://kafka.apache.org/documentation/)
- [rdkafka Crate Documentation](https://docs.rs/rdkafka/)
- [Confluent Kafka Documentation](https://docs.confluent.io/)
- [Kafka Consumer Best Practices](https://kafka.apache.org/documentation/#consumerconfigs)
- [Schema Registry](https://docs.confluent.io/platform/current/schema-registry/index.html)

## 🔧 Solução de Problemas

### Problemas Comuns

1. **Falha de Conexão**
   ```
   Erro: Falha ao conectar ao broker Kafka
   ```
   - Verificar se o Kafka está rodando
   - Confirmar endereço e porta do broker
   - Verificar conectividade de rede

2. **Erro de Autenticação**
   ```
   Erro: SASL authentication failed
   ```
   - Verificar credenciais API
   - Confirmar configuração SASL/SSL
   - Verificar permissões do usuário

3. **Timeout de Sessão**
   ```
   Erro: Session timeout
   ```
   - Aumentar `session_timeout_ms`
   - Verificar latência de rede
   - Reduzir `max_poll_records`

4. **Consumer Lag**
   ```
   Warning: Consumer lag detectado
   ```
   - Aumentar paralelismo do processamento
   - Otimizar handlers
   - Considerar processamento em lote

### Debug e Logs

```rust
// Habilitar logs detalhados
use tracing::{info, warn, error};

consumer.subscribe(EventType::TokenSwap)
    .with_handler(|event| async move {
        info!("Processando evento: {:?}", event);
        
        match process_event(&event).await {
            Ok(_) => info!("Evento processado com sucesso"),
            Err(e) => error!("Erro ao processar evento: {}", e),
        }
    })
    .start()
    .await?;
```
