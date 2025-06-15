/*!
 * Ethernity DeepTrace - Analyzer
 *
 * Analisador principal de traces de transações
 */

use crate::memory::memory::MemoryManager;
use crate::{trace::*, ContractCreation, ContractType, ExecutionStep, TokenTransfer, TokenType, TraceAnalysisConfig};
use ethereum_types::{Address, H256, U256};
use std::sync::Arc;

/// Contexto de análise
pub struct AnalysisContext {
    pub tx_hash: H256,
    pub block_number: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub rpc_client: Arc<dyn ethernity_core::traits::RpcProvider>,
    pub memory_manager: Arc<MemoryManager>,
    pub config: TraceAnalysisConfig,
}

/// Analisador de traces
pub struct TraceAnalyzer {
    context: AnalysisContext,
}

impl TraceAnalyzer {
    /// Cria um novo analisador
    pub fn new(context: AnalysisContext) -> Self {
        Self { context }
    }

    /// Analisa um trace e recibo de transação
    pub async fn analyze(
        &self,
        trace: &CallTrace,
        receipt: &serde_json::Value,
    ) -> Result<TraceAnalysisResult, ()> {
        // Constrói a árvore de chamadas
        let call_tree = self.build_call_tree(trace)?;

        // Extrai transferências de tokens
        let token_transfers = self.extract_token_transfers(trace, receipt).await?;

        // Extrai criações de contratos
        let contract_creations = self.extract_contract_creations(trace)?;

        // Constrói o caminho de execução
        let execution_path = self.build_execution_path(trace)?;

        Ok(TraceAnalysisResult {
            call_tree,
            token_transfers,
            contract_creations,
            execution_path,
        })
    }

    /// Constrói a árvore de chamadas
    fn build_call_tree(&self, trace: &CallTrace) -> Result<CallTree, ()> {
        let mut nodes = Vec::new();
        self.build_call_tree_recursive(trace, 0, &mut nodes)?;

        Ok(CallTree {
            root: CallNode {
                index: 0,
                depth: 0,
                call_type: trace.call_type.as_deref().map(CallType::from).unwrap_or(CallType::Call),
                from: Address::from_slice(&hex::decode(trace.from.trim_start_matches("0x")).unwrap_or_default()),
                to: if trace.to.is_empty() { None } else {
                    Some(Address::from_slice(&hex::decode(trace.to.trim_start_matches("0x")).unwrap_or_default()))
                },
                value: U256::from_dec_str(&trace.value).unwrap_or(U256::zero()),
                gas: U256::from_dec_str(&trace.gas).unwrap_or(U256::zero()),
                gas_used: U256::from_dec_str(&trace.gas_used).unwrap_or(U256::zero()),
                input: hex::decode(trace.input.trim_start_matches("0x")).unwrap_or_default(),
                output: hex::decode(trace.output.trim_start_matches("0x")).unwrap_or_default(),
                error: trace.error.clone(),
                children: Vec::new(),
            },
        })
    }

    /// Constrói a árvore de chamadas recursivamente
    fn build_call_tree_recursive(
        &self,
        trace: &CallTrace,
        depth: usize,
        nodes: &mut Vec<CallTreeNode>,
    ) -> Result<(), ()> {
        // Verifica limite de profundidade
        if depth > self.context.config.max_depth {
            return Ok(());
        }

        // Cria o nó atual
        let node = CallTreeNode {
            call: trace.clone(),
            depth,
            children: Vec::new(),
        };

        let node_index = nodes.len();
        nodes.push(node);

        // Processa chamadas filhas
        if let Some(calls) = &trace.calls {
            for child_call in calls {
                let child_index = nodes.len();
                self.build_call_tree_recursive(child_call, depth + 1, nodes)?;

                // Adiciona o índice do filho ao nó pai
                if let Some(parent_node) = nodes.get_mut(node_index) {
                    parent_node.children.push(child_index);
                }
            }
        }

        Ok(())
    }

    /// Extrai transferências de tokens dos logs
    async fn extract_token_transfers(
        &self,
        _trace: &CallTrace,
        receipt: &serde_json::Value,
    ) -> Result<Vec<TokenTransfer>, ()> {
        let mut transfers = Vec::new();

        // Processa logs do recibo
        if let Some(logs) = receipt.get("logs").and_then(|l| l.as_array()) {
            for (log_index, log) in logs.iter().enumerate() {
                if let Some(transfer) = self.parse_token_transfer_log(log, log_index).await? {
                    transfers.push(transfer);
                }
            }
        }

        Ok(transfers)
    }

    /// Analisa um log para detectar transferência de token
    async fn parse_token_transfer_log(
        &self,
        log: &serde_json::Value,
        call_index: usize,
    ) -> Result<Option<TokenTransfer>, ()> {
        // Verifica se é um evento Transfer ERC20/ERC721
        if let Some(topics) = log.get("topics").and_then(|t| t.as_array()) {
            if topics.len() >= 3 {
                let transfer_signature = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

                if let Some(topic0) = topics[0].as_str() {
                    if topic0 == transfer_signature {
                        // Extrai endereços from e to
                        let from = Address::from_slice(&hex::decode(
                            topics[1].as_str().unwrap_or("").trim_start_matches("0x")
                        ).unwrap_or_default());

                        let to = Address::from_slice(&hex::decode(
                            topics[2].as_str().unwrap_or("").trim_start_matches("0x")
                        ).unwrap_or_default());

                        // Determina o tipo de token e valor
                        let (token_type, amount, token_id) = if topics.len() == 4 {
                            // ERC721 - token_id no terceiro tópico
                            let token_id = U256::from_str_radix(
                                topics[3].as_str().unwrap_or("0").trim_start_matches("0x"), 16
                            ).unwrap_or(U256::zero());
                            (TokenType::Erc721, U256::one(), Some(token_id))
                        } else if let Some(data) = log.get("data").and_then(|d| d.as_str()) {
                            // ERC20 - valor nos dados
                            let amount = U256::from_str_radix(
                                data.trim_start_matches("0x"), 16
                            ).unwrap_or(U256::zero());
                            (TokenType::Erc20, amount, None)
                        } else {
                            return Ok(None);
                        };

                        let token_address = Address::from_slice(&hex::decode(
                            log.get("address").and_then(|a| a.as_str()).unwrap_or("").trim_start_matches("0x")
                        ).unwrap_or_default());

                        return Ok(Some(TokenTransfer {
                            token_type,
                            token_address,
                            from,
                            to,
                            amount,
                            token_id,
                            call_index,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Extrai criações de contratos
    fn extract_contract_creations(&self, trace: &CallTrace) -> Result<Vec<ContractCreation>, ()> {
        let mut creations = Vec::new();
        self.extract_contract_creations_recursive(trace, 0, &mut creations)?;
        Ok(creations)
    }

    /// Extrai criações de contratos recursivamente
    fn extract_contract_creations_recursive(
        &self,
        trace: &CallTrace,
        call_index: usize,
        creations: &mut Vec<ContractCreation>,
    ) -> Result<(), ()> {
        // Verifica se é uma criação de contrato
        let call_type = trace.call_type.as_deref().map(CallType::from).unwrap_or(CallType::Call);

        if call_type == CallType::Create || call_type == CallType::Create2 {
            if !trace.output.is_empty() {
                // Determina o tipo de contrato analisando o bytecode
                let output_bytes = hex::decode(trace.output.trim_start_matches("0x")).unwrap_or_default();
                let contract_type = self.determine_contract_type(&output_bytes)?;

                let from = Address::from_slice(&hex::decode(trace.from.trim_start_matches("0x")).unwrap_or_default());
                let contract_address = if trace.to.is_empty() {
                    Address::zero()
                } else {
                    Address::from_slice(&hex::decode(trace.to.trim_start_matches("0x")).unwrap_or_default())
                };

                let creation = ContractCreation {
                    creator: from,
                    contract_address,
                    init_code: hex::decode(trace.input.trim_start_matches("0x")).unwrap_or_default(),
                    contract_type,
                    call_index,
                };

                creations.push(creation);
            }
        }

        // Processa chamadas filhas
        if let Some(calls) = &trace.calls {
            for (i, child_call) in calls.iter().enumerate() {
                self.extract_contract_creations_recursive(child_call, call_index + i + 1, creations)?;
            }
        }

        Ok(())
    }

    /// Determina o tipo de contrato baseado no bytecode
    fn determine_contract_type(&self, bytecode: &[u8]) -> Result<ContractType, ()> {
        // Assinaturas de função conhecidas
        let erc20_signatures = [
            &[0x70, 0xa0, 0x82, 0x31], // balanceOf(address)
            &[0xa9, 0x05, 0x9c, 0xbb], // transfer(address,uint256)
            &[0x18, 0x16, 0x0d, 0xdd], // totalSupply()
        ];

        let erc721_signatures = [
            &[0x6f, 0xdd, 0x43, 0xe1], // balanceOf(address)
            &[0x6e, 0xb6, 0x1d, 0x3e], // ownerOf(uint256)
            &[0x42, 0x84, 0x2e, 0x0e], // safeTransferFrom(address,address,uint256)
        ];

        // Verifica assinaturas ERC20
        let mut erc20_count = 0;
        for signature in &erc20_signatures {
            if bytecode.windows(4).any(|window| window == *signature) {
                erc20_count += 1;
            }
        }

        if erc20_count >= 2 {
            return Ok(ContractType::Erc20Token);
        }

        // Verifica assinaturas ERC721
        let mut erc721_count = 0;
        for signature in &erc721_signatures {
            if bytecode.windows(4).any(|window| window == *signature) {
                erc721_count += 1;
            }
        }

        if erc721_count >= 2 {
            return Ok(ContractType::Erc721Token);
        }

        // Verifica padrões de proxy
        let proxy_patterns = [
            &[0x36, 0x3d, 0x3d, 0x37], // DELEGATECALL pattern
            &[0x5c, 0x60, 0x20, 0x60], // Minimal proxy pattern
        ];

        for pattern in &proxy_patterns {
            if bytecode.windows(4).any(|window| window == *pattern) {
                return Ok(ContractType::Proxy);
            }
        }

        // Verifica padrões de factory
        if bytecode.windows(4).any(|window| window == &[0xf0, 0x00, 0x00, 0x00]) { // CREATE opcode
            return Ok(ContractType::Factory);
        }

        Ok(ContractType::Unknown)
    }

    /// Constrói o caminho de execução
    fn build_execution_path(&self, trace: &CallTrace) -> Result<Vec<ExecutionStep>, ()> {
        let mut path = Vec::new();
        self.build_execution_path_recursive(trace, 0, &mut path)?;
        Ok(path)
    }

    /// Constrói o caminho de execução recursivamente
    fn build_execution_path_recursive(
        &self,
        trace: &CallTrace,
        depth: usize,
        path: &mut Vec<ExecutionStep>,
    ) -> Result<(), ()> {
        // Verifica limite de profundidade
        if depth > self.context.config.max_depth {
            return Ok(());
        }

        // Adiciona o passo atual
        let step = ExecutionStep {
            depth,
            call_type: trace.call_type.as_deref().map(CallType::from).unwrap_or(CallType::Call),
            from: Address::from_slice(&hex::decode(trace.from.trim_start_matches("0x")).unwrap_or_default()),
            to: if trace.to.is_empty() {
                Address::zero()
            } else {
                Address::from_slice(&hex::decode(trace.to.trim_start_matches("0x")).unwrap_or_default())
            },
            value: U256::from_dec_str(&trace.value).unwrap_or(U256::zero()),
            input: hex::decode(trace.input.trim_start_matches("0x")).unwrap_or_default(),
            output: hex::decode(trace.output.trim_start_matches("0x")).unwrap_or_default(),
            gas_used: U256::from_dec_str(&trace.gas_used).unwrap_or(U256::zero()),
            error: trace.error.clone(),
        };

        path.push(step);

        // Processa chamadas filhas
        if let Some(calls) = &trace.calls {
            for child_call in calls {
                self.build_execution_path_recursive(child_call, depth + 1, path)?;
            }
        }

        Ok(())
    }
}

/// Resultado da análise de trace
pub struct TraceAnalysisResult {
    pub call_tree: CallTree,
    pub token_transfers: Vec<TokenTransfer>,
    pub contract_creations: Vec<ContractCreation>,
    pub execution_path: Vec<ExecutionStep>,
}

/// Nó da árvore de chamadas
pub struct CallTreeNode {
    pub call: CallTrace,
    pub depth: usize,
    pub children: Vec<usize>, // Índices dos nós filhos
}

impl CallTree {
    /// Obtém todos os nós em uma profundidade específica
    pub fn nodes_at_depth(&self, depth: usize) -> Vec<&CallNode> {
        let mut nodes = Vec::new();
        self.collect_nodes_at_depth(&self.root, depth, &mut nodes);
        nodes
    }

    fn collect_nodes_at_depth<'a>(&self, node: &'a CallNode, target_depth: usize, nodes: &mut Vec<&'a CallNode>) {
        if node.depth == target_depth {
            nodes.push(node);
        }
        for child in &node.children {
            self.collect_nodes_at_depth(child, target_depth, nodes);
        }
    }

    /// Obtém todas as chamadas que falharam
    pub fn failed_calls(&self) -> Vec<&CallNode> {
        let mut failed = Vec::new();
        self.collect_failed_calls(&self.root, &mut failed);
        failed
    }

    fn collect_failed_calls<'a>(&self, node: &'a CallNode, failed: &mut Vec<&'a CallNode>) {
        if node.error.is_some() {
            failed.push(node);
        }
        for child in &node.children {
            self.collect_failed_calls(child, failed);
        }
    }

    /// Obtém todas as chamadas para um endereço específico
    pub fn calls_to_address(&self, address: &Address) -> Vec<&CallNode> {
        let mut calls = Vec::new();
        self.collect_calls_to_address(&self.root, address, &mut calls);
        calls
    }

    fn collect_calls_to_address<'a>(&self, node: &'a CallNode, address: &Address, calls: &mut Vec<&'a CallNode>) {
        if node.to.map_or(false, |to| to == *address) {
            calls.push(node);
        }
        for child in &node.children {
            self.collect_calls_to_address(child, address, calls);
        }
    }

    /// Obtém todas as chamadas de um endereço específico
    pub fn calls_from_address(&self, address: &Address) -> Vec<&CallNode> {
        let mut calls = Vec::new();
        self.collect_calls_from_address(&self.root, address, &mut calls);
        calls
    }

    fn collect_calls_from_address<'a>(&self, node: &'a CallNode, address: &Address, calls: &mut Vec<&'a CallNode>) {
        if node.from == *address {
            calls.push(node);
        }
        for child in &node.children {
            self.collect_calls_from_address(child, address, calls);
        }
    }
}

/// Estatísticas de análise
#[derive(Debug)]
pub struct AnalysisStats {
    pub total_calls: usize,
    pub failed_calls: usize,
    pub max_depth: usize,
    pub token_transfers: usize,
    pub contract_creations: usize,
    pub unique_addresses: usize,
    pub total_gas_used: U256,
    pub analysis_time_ms: u64,
}

impl TraceAnalysisResult {
    /// Calcula estatísticas da análise
    pub fn calculate_stats(&self, analysis_time_ms: u64) -> AnalysisStats {
        let total_calls = self.call_tree.total_calls();
        let failed_calls = self.call_tree.failed_calls().len();
        let max_depth = self.call_tree.max_depth();
        let token_transfers = self.token_transfers.len();
        let contract_creations = self.contract_creations.len();

        // Calcula endereços únicos
        let mut unique_addresses = std::collections::HashSet::new();
        self.call_tree.traverse_preorder(|node| {
            unique_addresses.insert(node.from);
            if let Some(to) = node.to {
                unique_addresses.insert(to);
            }
        });

        // Calcula gas total usado
        let total_gas_used = self.execution_path.iter()
            .map(|step| step.gas_used)
            .fold(U256::zero(), |acc, gas| acc + gas);

        AnalysisStats {
            total_calls,
            failed_calls,
            max_depth,
            token_transfers,
            contract_creations,
            unique_addresses: unique_addresses.len(),
            total_gas_used,
            analysis_time_ms,
        }
    }
}