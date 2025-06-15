use async_trait::async_trait;
use ethereum_types::{Address, U256};
use ethernity_core::Error;
use std::str::FromStr;

/// Estrutura de trace de chamada
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CallTrace {
    pub from: String,
    pub gas: String,
    #[serde(rename = "gasUsed")]
    pub gas_used: String,
    pub to: String,
    pub input: String,
    pub output: String,
    pub value: String,
    pub error: Option<String>,
    pub calls: Option<Vec<CallTrace>>,
    #[serde(rename = "type")]
    pub call_type: Option<String>,
}

/// Tipo de chamada
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallType {
    Call,
    StaticCall,
    DelegateCall,
    CallCode,
    Create,
    Create2,
    SelfDestruct,
    Unknown,
}

impl From<&str> for CallType {
    fn from(s: &str) -> Self {
        match s {
            "CALL" => CallType::Call,
            "STATICCALL" => CallType::StaticCall,
            "DELEGATECALL" => CallType::DelegateCall,
            "CALLCODE" => CallType::CallCode,
            "CREATE" => CallType::Create,
            "CREATE2" => CallType::Create2,
            "SELFDESTRUCT" => CallType::SelfDestruct,
            _ => CallType::Unknown,
        }
    }
}

/// Árvore de chamadas
#[derive(Debug, Clone)]
pub struct CallTree {
    pub root: CallNode,
}

/// Nó da árvore de chamadas
#[derive(Debug, Clone)]
pub struct CallNode {
    pub index: usize,
    pub depth: usize,
    pub call_type: CallType,
    pub from: Address,
    pub to: Option<Address>,
    pub value: U256,
    pub gas: U256,
    pub gas_used: U256,
    pub input: Vec<u8>,
    pub output: Vec<u8>,
    pub error: Option<String>,
    pub children: Vec<CallNode>,
}

impl CallTree {
    /// Cria uma nova árvore de chamadas a partir de um trace
    pub fn from_trace(trace: &CallTrace) -> Result<Self, ()> {
        let mut index = 0;
        let root = Self::build_node(trace, 0, &mut index)?;

        Ok(Self { root })
    }

    /// Constrói um nó da árvore recursivamente
    fn build_node(trace: &CallTrace, depth: usize, index: &mut usize) -> Result<CallNode, ()> {
        let current_index = *index;
        *index += 1;

        // Converte os campos do trace
        let from = Address::from_str(&trace.from.trim_start_matches("0x"))
            .map_err(|_| Error::DecodeError(format!("Endereço inválido: {}", trace.from))).expect("ERR ");

        let to = if trace.to.is_empty() {
            None
        } else {
            Some(Address::from_str(&trace.to.trim_start_matches("0x"))
                .map_err(|_| Error::DecodeError(format!("Endereço inválido: {}", trace.to))).expect("ERR "))
        };

        let value = U256::from_dec_str(&trace.value)
            .map_err(|_| Error::DecodeError(format!("Valor inválido: {}", trace.value))).expect("ERR ");

        let gas = U256::from_dec_str(&trace.gas)
            .map_err(|_| Error::DecodeError(format!("Gas inválido: {}", trace.gas))).expect("ERR");

        let gas_used = U256::from_dec_str(&trace.gas_used)
            .map_err(|_| Error::DecodeError(format!("Gas usado inválido: {}", trace.gas_used))).expect("ERR");

        let input = hex::decode(trace.input.trim_start_matches("0x"))
            .map_err(|_| Error::DecodeError(format!("Input inválido: {}", trace.input))).expect("ERR");

        let output = hex::decode(trace.output.trim_start_matches("0x"))
            .map_err(|_| Error::DecodeError(format!("Output inválido: {}", trace.output))).expect("ERR");

        let call_type = trace.call_type.as_deref().map(CallType::from).unwrap_or(CallType::Call);

        // Processa os filhos recursivamente
        let mut children = Vec::new();

        if let Some(calls) = &trace.calls {
            for call in calls {
                children.push(Self::build_node(call, depth + 1, index)?);
            }
        }

        Ok(CallNode {
            index: current_index,
            depth,
            call_type,
            from,
            to,
            value,
            gas,
            gas_used,
            input,
            output,
            error: trace.error.clone(),
            children,
        })
    }

    /// Percorre a árvore em pré-ordem
    pub fn traverse_preorder<F>(&self, mut f: F)
    where
        F: FnMut(&CallNode),
    {
        self.traverse_preorder_node(&self.root, &mut f);
    }

    /// Percorre um nó em pré-ordem
    fn traverse_preorder_node<F>(&self, node: &CallNode, f: &mut F)
    where
        F: FnMut(&CallNode),
    {
        f(node);

        for child in &node.children {
            self.traverse_preorder_node(child, f);
        }
    }

    /// Percorre a árvore em pós-ordem
    pub fn traverse_postorder<F>(&self, mut f: F)
    where
        F: FnMut(&CallNode),
    {
        self.traverse_postorder_node(&self.root, &mut f);
    }

    /// Percorre um nó em pós-ordem
    fn traverse_postorder_node<F>(&self, node: &CallNode, f: &mut F)
    where
        F: FnMut(&CallNode),
    {
        for child in &node.children {
            self.traverse_postorder_node(child, f);
        }

        f(node);
    }

    /// Encontra um nó pelo índice
    pub fn find_by_index(&self, index: usize) -> Option<&CallNode> {
        self.find_by_index_node(&self.root, index)
    }

    /// Encontra um nó pelo índice a partir de um nó
    fn find_by_index_node<'a>(&self, node: &'a CallNode, index: usize) -> Option<&'a CallNode> {
        if node.index == index {
            return Some(node);
        }

        for child in &node.children {
            if let Some(found) = self.find_by_index_node(child, index) {
                return Some(found);
            }
        }

        None
    }

    /// Obtém o caminho até um nó
    pub fn path_to_node(&self, index: usize) -> Option<Vec<usize>> {
        let mut path = Vec::new();
        if self.path_to_node_rec(&self.root, index, &mut path) {
            Some(path)
        } else {
            None
        }
    }

    /// Obtém o caminho até um nó recursivamente
    fn path_to_node_rec(&self, node: &CallNode, target_index: usize, path: &mut Vec<usize>) -> bool {
        path.push(node.index);

        if node.index == target_index {
            return true;
        }

        for child in &node.children {
            if self.path_to_node_rec(child, target_index, path) {
                return true;
            }
        }

        path.pop();
        false
    }

    /// Obtém a profundidade máxima da árvore
    pub fn max_depth(&self) -> usize {
        self.max_depth_node(&self.root)
    }

    /// Obtém a profundidade máxima a partir de um nó
    fn max_depth_node(&self, node: &CallNode) -> usize {
        if node.children.is_empty() {
            return node.depth;
        }

        node.children.iter()
            .map(|child| self.max_depth_node(child))
            .max()
            .unwrap_or(node.depth)
    }

    /// Conta o número total de nós na árvore
    pub fn total_calls(&self) -> usize {
        let mut count = 0;
        self.traverse_preorder(|_| count += 1);
        count
    }

    /// Filtra nós com base em um predicado
    pub fn filter_nodes<F>(&self, mut predicate: F) -> Vec<CallNode>
    where
        F: FnMut(&CallNode) -> bool,
    {
        let mut result = Vec::new();
        self.traverse_preorder(|node| {
            if predicate(node) {
                result.push(node.clone());
            }
        });
        result
    }
}

/// Detector de padrões em traces
#[async_trait]
pub trait TraceDetector: Send + Sync {
    /// Detecta padrões em um trace
    async fn detect(&self, trace: &CallTrace) -> Result<Vec<crate::DetectedPattern>, ()>;
}