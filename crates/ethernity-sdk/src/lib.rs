/*!
 * Ethernity SDK
 * 
 * SDKs para consumidores de eventos Ethernity
 */

use ethernity_core::{error::Result, types::*, Error};
use rdkafka::consumer::Consumer;
use rdkafka::Message;
use std::sync::Arc;

/// Configuração do consumidor de eventos
#[derive(Debug, Clone)]
pub struct ConsumerConfig {
    pub kafka_brokers: Vec<String>,
    pub consumer_group: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub auto_commit: bool,
    pub max_poll_records: i32,
    pub session_timeout_ms: i32,
}

impl ConsumerConfig {
    /// Cria um builder para a configuração
    pub fn builder() -> ConsumerConfigBuilder {
        ConsumerConfigBuilder::default()
    }
}

/// Builder para configuração do consumidor
#[derive(Debug, Default)]
pub struct ConsumerConfigBuilder {
    kafka_brokers: Option<Vec<String>>,
    consumer_group: Option<String>,
    api_key: Option<String>,
    api_secret: Option<String>,
    auto_commit: bool,
    max_poll_records: i32,
    session_timeout_ms: i32,
}

impl ConsumerConfigBuilder {
    /// Define os brokers Kafka
    pub fn kafka_brokers<S: Into<String>>(mut self, brokers: S) -> Self {
        self.kafka_brokers = Some(vec![brokers.into()]);
        self
    }
    
    /// Define o grupo de consumidores
    pub fn consumer_group<S: Into<String>>(mut self, group: S) -> Self {
        self.consumer_group = Some(group.into());
        self
    }
    
    /// Define a chave de API
    pub fn api_key<S: Into<String>>(mut self, key: S) -> Self {
        self.api_key = Some(key.into());
        self
    }
    
    /// Define o segredo de API
    pub fn api_secret<S: Into<String>>(mut self, secret: S) -> Self {
        self.api_secret = Some(secret.into());
        self
    }
    
    /// Define se deve fazer commit automático
    pub fn auto_commit(mut self, auto_commit: bool) -> Self {
        self.auto_commit = auto_commit;
        self
    }
    
    /// Define o número máximo de registros por poll
    pub fn max_poll_records(mut self, max_poll_records: i32) -> Self {
        self.max_poll_records = max_poll_records;
        self
    }
    
    /// Define o timeout da sessão
    pub fn session_timeout_ms(mut self, timeout_ms: i32) -> Self {
        self.session_timeout_ms = timeout_ms;
        self
    }
    
    /// Constrói a configuração
    pub fn build(self) -> Result<ConsumerConfig> {
        let kafka_brokers = self.kafka_brokers.ok_or_else(|| {
            Error::ValidationError("kafka_brokers é obrigatório".to_string())
        })?;
        
        let consumer_group = self.consumer_group.ok_or_else(|| {
            Error::ValidationError("consumer_group é obrigatório".to_string())
        })?;
        
        Ok(ConsumerConfig {
            kafka_brokers,
            consumer_group,
            api_key: self.api_key,
            api_secret: self.api_secret,
            auto_commit: self.auto_commit,
            max_poll_records: self.max_poll_records,
            session_timeout_ms: self.session_timeout_ms,
        })
    }
}

/// Consumidor de eventos Ethernity
pub struct EthernityConsumer {
    config: ConsumerConfig,
    consumer: Arc<rdkafka::consumer::StreamConsumer>,
    subscriptions: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Subscription>>>,
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    task_handle: std::sync::Arc<tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl EthernityConsumer {
    /// Cria um novo consumidor
    pub async fn new(config: ConsumerConfig) -> Result<Self> {
        // Configura o cliente Kafka
        let mut kafka_config = rdkafka::ClientConfig::new();
        
        // Define os brokers
        kafka_config.set("bootstrap.servers", &config.kafka_brokers.join(","));
        
        // Define o grupo de consumidores
        kafka_config.set("group.id", &config.consumer_group);
        
        // Define as credenciais se fornecidas
        if let Some(api_key) = &config.api_key {
            kafka_config.set("sasl.username", api_key);
        }
        
        if let Some(api_secret) = &config.api_secret {
            kafka_config.set("sasl.password", api_secret);
            kafka_config.set("security.protocol", "SASL_SSL");
            kafka_config.set("sasl.mechanism", "PLAIN");
        }
        
        // Define outras configurações
        kafka_config.set("auto.offset.reset", "earliest");
        kafka_config.set("enable.auto.commit", if config.auto_commit { "true" } else { "false" });
        kafka_config.set("max.poll.records", &config.max_poll_records.to_string());
        kafka_config.set("session.timeout.ms", &config.session_timeout_ms.to_string());
        
        // Cria o consumidor
        let consumer: rdkafka::consumer::StreamConsumer = kafka_config.create()
            .map_err(|e| Error::Other(format!("Erro ao criar consumidor Kafka: {}", e)))?;
        
        Ok(Self {
            config,
            consumer: Arc::new(consumer),
            subscriptions: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            task_handle: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        })
    }
    
    /// Inscreve-se em um tipo de evento
    pub fn subscribe(&self, event_type: EventType) -> SubscriptionBuilder {
        SubscriptionBuilder {
            event_type,
            consumer: self,
            filter: None,
            handler: None,
            topic: format!("ethernity.events.{}", event_type.to_string().to_lowercase()),
        }
    }
    
    /// Inicia o consumo de eventos
    pub async fn start(&self) -> Result<()> {
        // Verifica se já está rodando
        if self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        // Marca como rodando
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        
        // Obtém os tópicos inscritos
        let subscriptions = self.subscriptions.read().await;
        let topics: Vec<&str> = subscriptions.values()
            .map(|s| s.topic.as_str())
            .collect();
        
        if topics.is_empty() {
            return Err(Error::ValidationError("Nenhuma inscrição configurada".to_string()));
        }
        
        // Inscreve nos tópicos
        self.consumer.subscribe(&topics)
            .map_err(|e| Error::Other(format!("Erro ao inscrever nos tópicos: {}", e)))?;
        
        // Clona as referências necessárias para a task
        let consumer = self.consumer.clone();
        let subscriptions = self.subscriptions.clone();
        let running = self.running.clone();
        
        // Inicia a task de consumo
        let handle = tokio::spawn(async move {
            use futures::StreamExt;
            
            // Cria um stream a partir do consumidor
            let mut message_stream = consumer.stream();
            
            while running.load(std::sync::atomic::Ordering::SeqCst) {
                match message_stream.next().await {
                    Some(Ok(message)) => {
                        // Processa a mensagem
                        if let Some(payload) = message.payload() {
                            if let Ok(payload_str) = std::str::from_utf8(payload) {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload_str) {
                                    // Obtém o tópico
                                    if let Some(topic) = message.topic() {
                                        // Processa com os handlers apropriados
                                        let subscriptions_guard = subscriptions.read().await;
                                        
                                        for subscription in subscriptions_guard.values() {
                                            if subscription.topic == topic {
                                                // Verifica o filtro
                                                let should_process = match &subscription.filter {
                                                    Some(filter) => filter(&json),
                                                    None => true,
                                                };
                                                
                                                if should_process {
                                                    // Processa com o handler
                                                    if let Some(handler) = &subscription.handler {
                                                        let json_clone = json.clone();
                                                        let handler_future = handler(json_clone);
                                                        
                                                        // Executa o handler
                                                        tokio::spawn(handler_future);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Faz commit se necessário
                        if !self.config.auto_commit {
                            consumer.commit_message(&message, rdkafka::consumer::CommitMode::Async)
                                .unwrap_or_else(|e| eprintln!("Erro ao fazer commit: {}", e));
                        }
                    },
                    Some(Err(e)) => {
                        eprintln!("Erro ao consumir mensagem: {}", e);
                    },
                    None => {
                        // Stream terminou
                        break;
                    }
                }
            }
        });
        
        // Armazena o handle da task
        let mut task_handle = self.task_handle.lock().await;
        *task_handle = Some(handle);
        
        Ok(())
    }
    
    /// Para o consumo de eventos
    pub async fn stop(&self) -> Result<()> {
        // Verifica se está rodando
        if !self.running.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(());
        }
        
        // Marca como não rodando
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        
        // Aguarda a task terminar
        let mut task_handle = self.task_handle.lock().await;
        if let Some(handle) = task_handle.take() {
            // Espera no máximo 5 segundos
            tokio::select! {
                result = handle => {
                    // Task terminou normalmente
                    if let Err(e) = result {
                        eprintln!("Erro na task do consumidor: {:?}", e);
                    }
                },
                _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                    // Timeout, mas o handle já foi consumido pelo select
                    eprintln!("Timeout ao parar consumidor");
                }
            }
        }
        
        Ok(())
    }
    
    /// Adiciona uma inscrição
    async fn add_subscription(&self, subscription: Subscription) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(subscription.id.clone(), subscription);
        Ok(())
    }
}

/// Inscrição em eventos
#[derive(Clone)]
struct Subscription {
    id: String,
    event_type: EventType,
    topic: String,
    filter: Option<std::sync::Arc<dyn Fn(&serde_json::Value) -> bool + Send + Sync>>,
    handler: Option<std::sync::Arc<dyn Fn(serde_json::Value) -> futures::future::BoxFuture<'static, ()> + Send + Sync>>,
}

/// Builder para inscrição em eventos
pub struct SubscriptionBuilder<'a> {
    event_type: EventType,
    consumer: &'a EthernityConsumer,
    filter: Option<std::sync::Arc<dyn Fn(&serde_json::Value) -> bool + Send + Sync>>,
    handler: Option<std::sync::Arc<dyn Fn(serde_json::Value) -> futures::future::BoxFuture<'static, ()> + Send + Sync>>,
    topic: String,
}

impl<'a> SubscriptionBuilder<'a> {
    /// Define um filtro para eventos
    pub fn with_filter<F>(mut self, filter: F) -> Self 
    where 
        F: Fn(&serde_json::Value) -> bool + Send + Sync + 'static 
    {
        self.filter = Some(std::sync::Arc::new(filter));
        self
    }
    
    /// Define um handler para eventos
    pub fn with_handler<F, Fut>(mut self, handler: F) -> Self 
    where 
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: futures::Future<Output = ()> + Send + 'static,
    {
        let handler_boxed = move |json: serde_json::Value| {
            let fut = handler(json);
            Box::pin(fut) as futures::future::BoxFuture<'static, ()>
        };
        
        self.handler = Some(std::sync::Arc::new(handler_boxed));
        self
    }
    
    /// Inicia a inscrição
    pub async fn start(self) -> Result<()> {
        // Verifica se há um handler
        if self.handler.is_none() {
            return Err(Error::ValidationError("Handler não configurado".to_string()));
        }
        
        // Cria a inscrição
        let subscription = Subscription {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: self.event_type,
            topic: self.topic,
            filter: self.filter,
            handler: self.handler,
        };
        
        // Adiciona a inscrição
        self.consumer.add_subscription(subscription).await?;
        
        // Se o consumidor já estiver rodando, reinicia para aplicar a nova inscrição
        if self.consumer.running.load(std::sync::atomic::Ordering::SeqCst) {
            self.consumer.stop().await?;
            self.consumer.start().await?;
        }
        
        Ok(())
    }
}
