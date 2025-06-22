use std::str::FromStr;
use ethereum_types::{Address, U256};
use ethernity_core::Error;
use super::{CallTrace, CallType};

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

impl CallTree {
    /// Obtém todos os nós em uma profundidade específica
    pub fn nodes_at_depth(&self, depth: usize) -> Vec<&CallNode> {
        let mut nodes = Vec::new();
        Self::collect_nodes_at_depth(&self.root, depth, &mut nodes);
        nodes
    }

    fn collect_nodes_at_depth<'a>(node: &'a CallNode, target_depth: usize, nodes: &mut Vec<&'a CallNode>) {
        if node.depth == target_depth {
            nodes.push(node);
        }
        for child in &node.children {
            Self::collect_nodes_at_depth(child, target_depth, nodes);
        }
    }

    /// Obtém todas as chamadas que falharam
    pub fn failed_calls(&self) -> Vec<&CallNode> {
        let mut failed = Vec::new();
        Self::collect_failed_calls(&self.root, &mut failed);
        failed
    }

    fn collect_failed_calls<'a>(node: &'a CallNode, failed: &mut Vec<&'a CallNode>) {
        if node.error.is_some() {
            failed.push(node);
        }
        for child in &node.children {
            Self::collect_failed_calls(child, failed);
        }
    }

    /// Obtém todas as chamadas para um endereço específico
    pub fn calls_to_address(&self, address: &ethereum_types::Address) -> Vec<&CallNode> {
        let mut calls = Vec::new();
        Self::collect_calls_to_address(&self.root, address, &mut calls);
        calls
    }

    fn collect_calls_to_address<'a>(node: &'a CallNode, address: &ethereum_types::Address, calls: &mut Vec<&'a CallNode>) {
        if node.to.map_or(false, |to| to == *address) {
            calls.push(node);
        }
        for child in &node.children {
            Self::collect_calls_to_address(child, address, calls);
        }
    }

    /// Obtém todas as chamadas de um endereço específico
    pub fn calls_from_address(&self, address: &ethereum_types::Address) -> Vec<&CallNode> {
        let mut calls = Vec::new();
        Self::collect_calls_from_address(&self.root, address, &mut calls);
        calls
    }

    fn collect_calls_from_address<'a>(node: &'a CallNode, address: &ethereum_types::Address, calls: &mut Vec<&'a CallNode>) {
        if node.from == *address {
            calls.push(node);
        }
        for child in &node.children {
            Self::collect_calls_from_address(child, address, calls);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_addr(n: u64) -> String {
        format!("0x{:040x}", n)
    }

    fn make_trace() -> CallTrace {
        CallTrace {
            from: hex_addr(1),
            gas: "0".into(),
            gas_used: "0".into(),
            to: hex_addr(2),
            input: "0x".into(),
            output: "0x".into(),
            value: "0".into(),
            error: None,
            calls: Some(vec![CallTrace{
                from: hex_addr(1),
                gas: "0".into(),
                gas_used: "0".into(),
                to: hex_addr(3),
                input: "0x".into(),
                output: "0x".into(),
                value: "0".into(),
                error: Some("err".into()),
                calls: None,
                call_type: Some("CALL".into()),
            }]),
            call_type: Some("CALL".into()),
        }
    }

    #[test]
    fn test_private_helpers() {
        let trace = make_trace();
        let mut idx = 0;
        let node = CallTree::build_node(&trace, 0, &mut idx).unwrap();
        let tree = CallTree{root: node};

        let mut pre = Vec::new();
        tree.traverse_preorder_node(&tree.root, &mut |n| pre.push(n.index));
        assert_eq!(pre, vec![0,1]);

        let mut post = Vec::new();
        tree.traverse_postorder_node(&tree.root, &mut |n| post.push(n.index));
        assert_eq!(post, vec![1,0]);

        assert!(tree.find_by_index_node(&tree.root, 1).is_some());
        assert!(tree.find_by_index_node(&tree.root, 99).is_none());

        let mut path = Vec::new();
        assert!(tree.path_to_node_rec(&tree.root, 1, &mut path));
        assert_eq!(path, vec![0,1]);

        assert_eq!(tree.max_depth_node(&tree.root), 1);

        let mut depth_nodes = Vec::new();
        CallTree::collect_nodes_at_depth(&tree.root, 1, &mut depth_nodes);
        assert_eq!(depth_nodes.len(), 1);

        let mut failed = Vec::new();
        CallTree::collect_failed_calls(&tree.root, &mut failed);
        assert_eq!(failed.len(), 1);

        let mut to_calls = Vec::new();
        CallTree::collect_calls_to_address(&tree.root, &Address::from_low_u64_be(3), &mut to_calls);
        assert_eq!(to_calls.len(), 1);

        let mut from_calls = Vec::new();
        CallTree::collect_calls_from_address(&tree.root, &Address::from_low_u64_be(1), &mut from_calls);
        assert_eq!(from_calls.len(), 2);
    }
}

