/*!
 * Ethernity RPC
 * 
 * Cliente RPC para interação com nodes Ethereum
 */

use ethernity_core::{Error, error::Result, types::*};
use ethereum_types::Address;
use web3::{
    Web3, Transport,
    transports::{Http, WebSocket},
    types::{Bytes, BlockNumber, BlockId, U64, H256 as Web3H256, H160},
};
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use parking_lot::RwLock;
use async_trait::async_trait;

/// Configuração do cliente RPC
#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub endpoint: String,
    pub timeout: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub use_cache: bool,
    pub cache_ttl: Duration,
    pub connection_pool_size: usize,
}

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

/// Enum para diferentes tipos de transporte
pub enum TransportType {
    Http(Web3<Http>),
    WebSocket(Web3<WebSocket>),
}

/// Cliente RPC para Ethereum
pub struct EthernityRpcClient {
    transport: TransportType,
    config: RpcConfig,
    cache: Arc<RwLock<HashMap<String, (Vec<u8>, std::time::Instant)>>>,
}

impl EthernityRpcClient {
    /// Cria um novo cliente RPC HTTP
    pub async fn new_http(config: RpcConfig) -> Result<Self> {
        let transport = Http::new(&config.endpoint)
            .map_err(|e| Error::RpcError(format!("Falha ao conectar via HTTP: {}", e)))?;
        
        let web3 = Web3::new(transport);
        
        // Verifica a conexão
        web3.eth().block_number()
            .await
            .map_err(|e| Error::RpcError(format!("Falha ao conectar ao node Ethereum: {}", e)))?;
        
        Ok(Self {
            transport: TransportType::Http(web3),
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Cria um novo cliente RPC WebSocket
    pub async fn new_websocket(config: RpcConfig) -> Result<Self> {
        let transport = WebSocket::new(&config.endpoint)
            .await
            .map_err(|e| Error::RpcError(format!("Falha ao conectar via WebSocket: {}", e)))?;
        
        let web3 = Web3::new(transport);
        
        // Verifica a conexão
        web3.eth().block_number()
            .await
            .map_err(|e| Error::RpcError(format!("Falha ao conectar ao node Ethereum: {}", e)))?;
        
        Ok(Self {
            transport: TransportType::WebSocket(web3),
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Cria um novo cliente baseado na URL
    pub async fn new(config: RpcConfig) -> Result<Self> {
        if config.endpoint.starts_with("ws") {
            Self::new_websocket(config).await
        } else {
            Self::new_http(config).await
        }
    }
    
    /// Obtém o trace de uma transação
    pub async fn get_transaction_trace(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        let cache_key = format!("trace_{:x}", tx_hash);
        
        // Verifica o cache
        if self.config.use_cache {
            let cache = self.cache.read();
            if let Some((data, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed() < self.config.cache_ttl {
                    return Ok(data.clone());
                }
            }
        }
        
        // Converte para o formato do web3
        let web3_hash = Web3H256::from_slice(tx_hash.as_bytes());
        
        // Parâmetros para debug_traceTransaction
        let params = vec![
            serde_json::Value::String(format!("{:?}", web3_hash)),
            serde_json::json!({
                "tracer": "callTracer",
                "timeout": "60s"
            })
        ];
        
        // Executa a chamada RPC diretamente
        let result = match &self.transport {
            TransportType::Http(web3) => {
                web3.transport().execute("debug_traceTransaction", params)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter trace da transação: {}", e)))?
            },
            TransportType::WebSocket(web3) => {
                web3.transport().execute("debug_traceTransaction", params)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter trace da transação: {}", e)))?
            }
        };
        
        // Converte o resultado para bytes
        let trace_bytes = serde_json::to_vec(&result)
            .map_err(|e| Error::EncodeError(format!("Falha ao serializar trace: {}", e)))?;
        
        // Atualiza o cache
        if self.config.use_cache {
            let mut cache = self.cache.write();
            cache.insert(cache_key, (trace_bytes.clone(), std::time::Instant::now()));
        }
        
        Ok(trace_bytes)
    }
    
    /// Obtém o recibo de uma transação
    pub async fn get_transaction_receipt(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        let cache_key = format!("receipt_{:x}", tx_hash);
        
        // Verifica o cache
        if self.config.use_cache {
            let cache = self.cache.read();
            if let Some((data, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed() < self.config.cache_ttl {
                    return Ok(data.clone());
                }
            }
        }
        
        // Converte para o formato do web3
        let web3_hash = Web3H256::from_slice(tx_hash.as_bytes());
        
        // Executa a chamada RPC diretamente
        let receipt = match &self.transport {
            TransportType::Http(web3) => {
                web3.eth().transaction_receipt(web3_hash)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter recibo da transação: {}", e)))?
            },
            TransportType::WebSocket(web3) => {
                web3.eth().transaction_receipt(web3_hash)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter recibo da transação: {}", e)))?
            }
        };
        
        let receipt = receipt.ok_or_else(|| Error::NotFound("Recibo da transação não encontrado".to_string()))?;
        
        // Converte o resultado para bytes
        let receipt_bytes = serde_json::to_vec(&receipt)
            .map_err(|e| Error::EncodeError(format!("Falha ao serializar recibo: {}", e)))?;
        
        // Atualiza o cache
        if self.config.use_cache {
            let mut cache = self.cache.write();
            cache.insert(cache_key, (receipt_bytes.clone(), std::time::Instant::now()));
        }
        
        Ok(receipt_bytes)
    }

    /// Obtém informações de um bloco
    pub async fn get_block(&self, block_number: u64) -> Result<Vec<u8>> {
        let cache_key = format!("block_{}", block_number);
        
        // Verifica o cache
        if self.config.use_cache {
            let cache = self.cache.read();
            if let Some((data, timestamp)) = cache.get(&cache_key) {
                if timestamp.elapsed() < self.config.cache_ttl {
                    return Ok(data.clone());
                }
            }
        }
        
        // Executa a chamada RPC diretamente
        let block = match &self.transport {
            TransportType::Http(web3) => {
                web3.eth().block(BlockId::Number(BlockNumber::Number(U64::from(block_number))))
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter bloco: {}", e)))?
            },
            TransportType::WebSocket(web3) => {
                web3.eth().block(BlockId::Number(BlockNumber::Number(U64::from(block_number))))
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter bloco: {}", e)))?
            }
        };
        
        let block = block.ok_or_else(|| Error::NotFound("Bloco não encontrado".to_string()))?;
        
        // Converte o resultado para bytes
        let block_bytes = serde_json::to_vec(&block)
            .map_err(|e| Error::EncodeError(format!("Falha ao serializar bloco: {}", e)))?;
        
        // Atualiza o cache
        if self.config.use_cache {
            let mut cache = self.cache.write();
            cache.insert(cache_key, (block_bytes.clone(), std::time::Instant::now()));
        }
        
        Ok(block_bytes)
    }

    /// Obtém o número do bloco atual
    pub async fn get_block_number(&self) -> Result<u64> {
        let block_number = match &self.transport {
            TransportType::Http(web3) => {
                web3.eth().block_number()
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter número do bloco: {}", e)))?
            },
            TransportType::WebSocket(web3) => {
                web3.eth().block_number()
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter número do bloco: {}", e)))?
            }
        };
        
        Ok(block_number.as_u64())
    }

    /// Obtém o código de um contrato
    pub async fn get_code(&self, address: Address) -> Result<Vec<u8>> {
        let result = match &self.transport {
            TransportType::Http(web3) => {
                web3.eth().code(H160::from_slice(address.as_bytes()), None)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter código do contrato: {}", e)))?
            },
            TransportType::WebSocket(web3) => {
                web3.eth().code(H160::from_slice(address.as_bytes()), None)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter código do contrato: {}", e)))?
            }
        };

        Ok(result.0)
    }

    /// Limpa o cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    /// Obtém estatísticas do cache
    pub fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read();
        let now = std::time::Instant::now();
        let mut expired = 0;
        
        for (_, (_, timestamp)) in cache.iter() {
            if now.duration_since(*timestamp) > self.config.cache_ttl {
                expired += 1;
            }
        }
        
        CacheStats {
            total_entries: cache.len(),
            expired_entries: expired,
            cache_hit_ratio: 0.0, // Seria necessário rastrear hits/misses
        }
    }
}

/// Implementação da trait RpcProvider do ethernity-core
#[async_trait]
impl ethernity_core::traits::RpcProvider for EthernityRpcClient {
    async fn get_transaction_trace(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        self.get_transaction_trace(tx_hash).await
    }

    async fn get_transaction_receipt(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        self.get_transaction_receipt(tx_hash).await
    }

    async fn get_code(&self, address: Address) -> Result<Vec<u8>> {
        let result = match &self.transport {
            TransportType::Http(web3) => {
                web3.eth().code(H160::from_slice(address.as_bytes()), None)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter código do contrato: {}", e)))?
            },
            TransportType::WebSocket(web3) => {
                web3.eth().code(H160::from_slice(address.as_bytes()), None)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha ao obter código do contrato: {}", e)))?
            }
        };

        Ok(result.0)
    }

    async fn call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>> {
        let call_request = web3::types::CallRequest {
            from: None,
            to: Some(H160::from_slice(to.as_bytes())),
            gas: None,
            gas_price: None,
            value: None,
            data: Some(Bytes(data)),
            transaction_type: None,
            access_list: None,
            max_fee_per_gas: None,
            max_priority_fee_per_gas: None,
        };

        let result = match &self.transport {
            TransportType::Http(web3) => {
                web3.eth().call(call_request, None)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha na chamada RPC: {}", e)))?
            },
            TransportType::WebSocket(web3) => {
                web3.eth().call(call_request, None)
                    .await
                    .map_err(|e| Error::RpcError(format!("Falha na chamada RPC: {}", e)))?
            }
        };

        Ok(result.0)
    }

    async fn get_block_number(&self) -> Result<u64> {
        self.get_block_number().await
    }
}

/// Estatísticas do cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub cache_hit_ratio: f64,
}

/// Pool de conexões RPC
pub struct RpcConnectionPool {
    clients: Vec<Arc<EthernityRpcClient>>,
    current_index: std::sync::atomic::AtomicUsize,
}

impl RpcConnectionPool {
    /// Cria um novo pool de conexões
    pub async fn new(config: RpcConfig, pool_size: usize) -> Result<Self> {
        let mut clients = Vec::with_capacity(pool_size);
        
        for _ in 0..pool_size {
            let client = Arc::new(EthernityRpcClient::new(config.clone()).await?);
            clients.push(client);
        }
        
        Ok(Self {
            clients,
            current_index: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// Obtém o próximo cliente do pool (round-robin)
    pub fn get_client(&self) -> Arc<EthernityRpcClient> {
        let index = self.current_index.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % self.clients.len();
        self.clients[index].clone()
    }

    /// Obtém estatísticas do pool
    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            total_clients: self.clients.len(),
            active_clients: self.clients.len(), // Simplificado - todos são considerados ativos
        }
    }
}

/// Estatísticas do pool de conexões
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_clients: usize,
    pub active_clients: usize,
}

/// Cliente RPC com balanceamento de carga
pub struct LoadBalancedRpcClient {
    pool: RpcConnectionPool,
}

impl LoadBalancedRpcClient {
    /// Cria um novo cliente com balanceamento de carga
    pub async fn new(config: RpcConfig) -> Result<Self> {
        let pool = RpcConnectionPool::new(config.clone(), config.connection_pool_size).await?;
        
        Ok(Self { pool })
    }
}

#[async_trait]
impl ethernity_core::traits::RpcProvider for LoadBalancedRpcClient {
    async fn get_transaction_trace(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        let client = self.pool.get_client();
        client.get_transaction_trace(tx_hash).await
    }

    async fn get_transaction_receipt(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        let client = self.pool.get_client();
        client.get_transaction_receipt(tx_hash).await
    }

    async fn get_code(&self, address: Address) -> Result<Vec<u8>> {
        let client = self.pool.get_client();
        client.get_code(address).await
    }

    async fn call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>> {
        let client = self.pool.get_client();
        client.call(to, data).await
    }

    async fn get_block_number(&self) -> Result<u64> {
        let client = self.pool.get_client();
        client.get_block_number().await
    }
}

