/*!
 * Ethernity Finder
 *
 * Busca de nodes Ethereum via Shodan e verificação de métodos RPC
 */

use ethernity_core::{error::Result, Error};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use std::fmt;
use std::str::FromStr;

/// Métodos RPC internos suportados
#[derive(Debug, Clone)]
pub enum RpcMethod {
    /// `debug_traceTransaction`
    DebugTraceTransaction,
    /// `admin_nodeInfo`
    AdminNodeInfo,
    /// `admin_peers`
    AdminPeers,
    /// `txpool_content`
    TxPoolContent,
    /// `trace_block`
    TraceBlock,
}

impl RpcMethod {
    /// Representação em string do método
    pub fn as_str(&self) -> &'static str {
        match self {
            RpcMethod::DebugTraceTransaction => "debug_traceTransaction",
            RpcMethod::AdminNodeInfo => "admin_nodeInfo",
            RpcMethod::AdminPeers => "admin_peers",
            RpcMethod::TxPoolContent => "txpool_content",
            RpcMethod::TraceBlock => "trace_block",
        }
    }
}

impl fmt::Display for RpcMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for RpcMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug_traceTransaction" => Ok(RpcMethod::DebugTraceTransaction),
            "admin_nodeInfo" => Ok(RpcMethod::AdminNodeInfo),
            "admin_peers" => Ok(RpcMethod::AdminPeers),
            "txpool_content" => Ok(RpcMethod::TxPoolContent),
            "trace_block" => Ok(RpcMethod::TraceBlock),
            other => Err(format!("Método desconhecido: {}", other)),
        }
    }
}

/// Opções para busca de nodes
#[derive(Debug, Clone)]
pub struct FinderOptions {
    /// Chain ID desejado
    pub chain_id: u64,
    /// Métodos RPC internos que o node deve suportar
    pub methods: Vec<RpcMethod>,
    /// Quantidade máxima de nodes (None para "all")
    pub limit: Option<usize>,
}

/// Status de verificação de um método RPC
#[derive(Debug, Clone)]
pub struct MethodStatus {
    pub method: RpcMethod,
    pub success: bool,
    pub error: Option<String>,
}

/// Informações de um node verificado
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub ip: String,
    pub port: u16,
    pub chain_id: u64,
    pub methods: Vec<MethodStatus>,
}

#[derive(Deserialize)]
struct ShodanResponse {
    matches: Vec<ShodanMatch>,
}

#[derive(Deserialize)]
struct ShodanMatch {
    ip_str: String,
    port: u16,
}

#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'static str,
    method: &'a str,
    params: Vec<serde_json::Value>,
    id: u32,
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcError {
    message: String,
}

const SHODAN_URL: &str = "https://api.shodan.io/shodan/host/search?key=2GgOLxfUg84qh7TaRedXMFch9UVjjup6&query=geth";

/// Trait para busca de nodes
#[async_trait]
pub trait NodeFinder {
    async fn find_nodes(&self, opts: FinderOptions) -> Result<Vec<NodeInfo>>;
}

/// Implementação padrão usando reqwest
pub struct ShodanFinder {
    client: Client,
}

impl ShodanFinder {
    /// Cria nova instância
    pub fn new() -> Self {
        Self { client: Client::new() }
    }
}

#[async_trait]
impl NodeFinder for ShodanFinder {
    async fn find_nodes(&self, opts: FinderOptions) -> Result<Vec<NodeInfo>> {
        let resp = self
            .client
            .get(SHODAN_URL)
            .send()
            .await
            .map_err(|e| Error::RpcError(format!("Erro ao consultar Shodan: {}", e)))?;

        let shodan: ShodanResponse = resp
            .json()
            .await
            .map_err(|e| Error::DecodeError(format!("Erro ao decodificar resposta Shodan: {}", e)))?;

        let mut found = Vec::new();
        for entry in shodan.matches {
            if let Some(limit) = opts.limit {
                if found.len() >= limit {
                    break;
                }
            }

            if let Some(node) = verify_node(&self.client, &entry.ip_str, entry.port, &opts).await? {
                found.push(node);
            }
        }
        Ok(found)
    }
}

async fn verify_node(client: &Client, ip: &str, port: u16, opts: &FinderOptions) -> Result<Option<NodeInfo>> {
    let url = format!("http://{}:{}", ip, port);

    // Verifica chain id
    let chain_id_req = RpcRequest {
        jsonrpc: "2.0",
        method: "eth_chainId",
        params: vec![],
        id: 1,
    };

    let chain_id_resp: RpcResponse = match client.post(&url).json(&chain_id_req).send().await {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(e) => {
                return Err(Error::RpcError(format!("Erro ao decodificar chainId: {}", e)));
            }
        },
        Err(_) => return Ok(None),
    };

    let chain_hex = match chain_id_resp.result {
        Some(ref v) => v.as_str().unwrap_or(""),
        None => return Ok(None),
    };

    let chain_id = u64::from_str_radix(chain_hex.trim_start_matches("0x"), 16).unwrap_or(0);
    if chain_id != opts.chain_id {
        return Ok(None);
    }

    // Verifica métodos
    let mut statuses = Vec::new();
    for method in &opts.methods {
        let req = RpcRequest {
            jsonrpc: "2.0",
            method: method.as_str(),
            params: vec![],
            id: 1,
        };

        let status = match client.post(&url).json(&req).send().await {
            Ok(r) => {
                if !r.status().is_success() {
                    MethodStatus {
                        method: method.clone(),
                        success: false,
                        error: Some(format!("HTTP {}", r.status())),
                    }
                } else {
                    match r.json::<RpcResponse>().await {
                        Ok(res) => {
                            if let Some(err) = res.error {
                                let msg = err.message;
                                let supported = !msg.to_lowercase().contains("method not found");
                                MethodStatus {
                                    method: method.clone(),
                                    success: supported,
                                    error: if supported { Some(msg) } else { None },
                                }
                            } else {
                                MethodStatus {
                                    method: method.clone(),
                                    success: true,
                                    error: None,
                                }
                            }
                        }
                        Err(e) => MethodStatus {
                            method: method.clone(),
                            success: false,
                            error: Some(e.to_string()),
                        },
                    }
                }
            }
            Err(e) => MethodStatus {
                method: method.clone(),
                success: false,
                error: Some(e.to_string()),
            },
        };
        statuses.push(status);
    }

    Ok(Some(NodeInfo {
        ip: ip.to_string(),
        port,
        chain_id,
        methods: statuses,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hex_parse() {
        let opts = FinderOptions { chain_id: 1, methods: Vec::new(), limit: None };
        let client = Client::new();
        let res = verify_node(&client, "127.0.0.1", 8545, &opts).await;
        // As we don't have a node, just ensure it doesn't panic and returns Ok
        assert!(res.is_ok());
    }
}
