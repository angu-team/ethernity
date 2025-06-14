/*!
 * Ethernity DeepTrace - Detectors
 *
 * Detectores especializados para diferentes tipos de eventos
 */

use crate::analyzer::TraceAnalysisResult;
use async_trait::async_trait;
use ethereum_types::{Address, U256};

/// Trait base para detectores especializados
#[async_trait]
pub trait SpecializedDetector: Send + Sync {
    /// Nome do detector
    fn name(&self) -> &str;

    /// Detecta eventos específicos na análise
    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()>;
}

/// Evento detectado por um detector especializado
#[derive(Debug, Clone)]
pub struct DetectedEvent {
    pub event_type: String,
    pub confidence: f64,
    pub addresses: Vec<Address>,
    pub data: serde_json::Value,
    pub description: String,
    pub severity: EventSeverity,
}

/// Severidade do evento
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Detector de sandwich attacks
pub struct SandwichAttackDetector;

impl SandwichAttackDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for SandwichAttackDetector {
    fn name(&self) -> &str {
        "SandwichAttackDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();

        // Sandwich attacks são caracterizados por:
        // 1. Múltiplas transferências do mesmo token
        // 2. Padrão de compra -> transação vítima -> venda
        // 3. Lucro líquido para o atacante

        if analysis.token_transfers.len() >= 3 {
            // Agrupa transferências por token
            let mut token_groups = std::collections::HashMap::new();
            for (i, transfer) in analysis.token_transfers.iter().enumerate() {
                token_groups.entry(transfer.token_address)
                    .or_insert_with(Vec::new)
                    .push((i, transfer));
            }

            for (token, transfers) in token_groups {
                if transfers.len() >= 3 {
                    // Analisa se há padrão de sandwich
                    for window in transfers.windows(3) {
                        if let [(i1, t1), (i2, t2), (i3, t3)] = window {
                            // Verifica se é o mesmo endereço fazendo compra e venda
                            if t1.to == t3.from && t1.from == t3.to {
                                // Verifica se há uma transação intermediária de outro usuário
                                if t2.from != t1.from && t2.to != t1.to {
                                    let profit = if t3.amount > t1.amount {
                                        t3.amount - t1.amount
                                    } else {
                                        U256::zero()
                                    };

                                    if profit > U256::zero() {
                                        let mut data = serde_json::Map::new();
                                        data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", token)));
                                        data.insert("attacker".to_string(), serde_json::Value::String(format!("{:?}", t1.from)));
                                        data.insert("victim".to_string(), serde_json::Value::String(format!("{:?}", t2.from)));
                                        data.insert("profit".to_string(), serde_json::Value::String(profit.to_string()));

                                        let event = DetectedEvent {
                                            event_type: "sandwich_attack".to_string(),
                                            confidence: 0.85,
                                            addresses: vec![token, t1.from, t2.from],
                                            data: serde_json::Value::Object(data),
                                            description: "Possível sandwich attack detectado".to_string(),
                                            severity: EventSeverity::High,
                                        };

                                        events.push(event);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }
}

/// Detector de frontrunning
pub struct FrontrunningDetector;

impl FrontrunningDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for FrontrunningDetector {
    fn name(&self) -> &str {
        "FrontrunningDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();

        // Frontrunning é detectado por:
        // 1. Transações similares em sequência
        // 2. Gas price mais alto na primeira transação
        // 3. Mesmo contrato/função sendo chamado

        // Coleta todos os nós da árvore de chamadas
        let mut call_nodes = Vec::new();
        analysis.call_tree.traverse_preorder(|node| {
            call_nodes.push(node.clone());
        });

        // Analisa chamadas similares
        for window in call_nodes.windows(2) {
            if let [call1, call2] = window {
                // Verifica se são chamadas para o mesmo contrato
                if call1.to == call2.to && call1.to.is_some() {
                    // Verifica se têm inputs similares (mesma função)
                    if call1.input.len() >= 4 && call2.input.len() >= 4 {
                        // Compara os primeiros 4 bytes (seletor de função)
                        if call1.input[0..4] == call2.input[0..4] {
                            // Verifica se são de endereços diferentes
                            if call1.from != call2.from {
                                let mut data = serde_json::Map::new();
                                data.insert("contract".to_string(), serde_json::Value::String(format!("{:?}", call1.to)));
                                data.insert("frontrunner".to_string(), serde_json::Value::String(format!("{:?}", call1.from)));
                                data.insert("victim".to_string(), serde_json::Value::String(format!("{:?}", call2.from)));
                                data.insert("function".to_string(), serde_json::Value::String(hex::encode(&call1.input[0..4])));

                                let event = DetectedEvent {
                                    event_type: "frontrunning".to_string(),
                                    confidence: 0.75,
                                    addresses: vec![call1.to.unwrap_or_else(|| Address::zero()), call1.from, call2.from],
                                    data: serde_json::Value::Object(data),
                                    description: "Possível frontrunning detectado".to_string(),
                                    severity: EventSeverity::Medium,
                                };

                                events.push(event);
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }
}

/// Detector de reentrancy attacks
pub struct ReentrancyDetector;

impl ReentrancyDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for ReentrancyDetector {
    fn name(&self) -> &str {
        "ReentrancyDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();

        // Reentrancy é detectado por:
        // 1. Chamadas recursivas para o mesmo contrato
        // 2. Múltiplas chamadas para a mesma função
        // 3. Padrão de call -> external call -> call

        let mut call_stack = std::collections::HashMap::new();

        analysis.call_tree.traverse_preorder(|node| {
            if let Some(to) = node.to {
                let key = (to, node.from);
                let count = call_stack.entry(key).or_insert(0);
                *count += 1;

                // Se há múltiplas chamadas do mesmo endereço para o mesmo contrato
                if *count > 1 {
                    // Verifica se há chamadas aninhadas (reentrancy)
                    let mut has_nested_calls = false;

                    // Percorre todos os nós para verificar se há chamadas aninhadas
                    analysis.call_tree.traverse_preorder(|other_node| {
                        if other_node.depth > node.depth &&
                            other_node.to == Some(node.from) {
                            has_nested_calls = true;
                        }
                    });

                    if has_nested_calls {
                        let mut data = serde_json::Map::new();
                        data.insert("contract".to_string(), serde_json::Value::String(format!("{:?}", to)));
                        data.insert("caller".to_string(), serde_json::Value::String(format!("{:?}", node.from)));
                        data.insert("call_count".to_string(), serde_json::Value::Number(serde_json::Number::from(*count)));

                        let event = DetectedEvent {
                            event_type: "reentrancy".to_string(),
                            confidence: 0.8,
                            addresses: vec![to, node.from],
                            data: serde_json::Value::Object(data),
                            description: "Possível reentrancy attack detectado".to_string(),
                            severity: EventSeverity::Critical,
                        };

                        events.push(event);
                    }
                }
            }
        });

        Ok(events)
    }
}

/// Detector de price manipulation
pub struct PriceManipulationDetector;

impl PriceManipulationDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for PriceManipulationDetector {
    fn name(&self) -> &str {
        "PriceManipulationDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();

        // Price manipulation é detectado por:
        // 1. Grandes transferências que afetam pools de liquidez
        // 2. Múltiplas operações em sequência no mesmo pool
        // 3. Padrão de manipulação -> operação -> restauração

        // Analisa transferências grandes em pools conhecidos
        for transfer in &analysis.token_transfers {
            // Verifica se é uma transferência grande (threshold arbitrário)
            if transfer.amount > U256::from(1000000) {
                // Procura por transferências subsequentes que possam indicar manipulação
                let mut related_transfers = Vec::new();
                for other_transfer in &analysis.token_transfers {
                    if other_transfer.token_address == transfer.token_address &&
                        other_transfer != transfer &&
                        (other_transfer.from == transfer.to || other_transfer.to == transfer.from) {
                        related_transfers.push(other_transfer);
                    }
                }

                if related_transfers.len() >= 2 {
                    let mut data = serde_json::Map::new();
                    data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", transfer.token_address)));
                    data.insert("manipulator".to_string(), serde_json::Value::String(format!("{:?}", transfer.from)));
                    data.insert("amount".to_string(), serde_json::Value::String(transfer.amount.to_string()));
                    data.insert("related_transfers".to_string(), serde_json::Value::Number(serde_json::Number::from(related_transfers.len())));

                    let event = DetectedEvent {
                        event_type: "price_manipulation".to_string(),
                        confidence: 0.7,
                        addresses: vec![transfer.token_address, transfer.from, transfer.to],
                        data: serde_json::Value::Object(data),
                        description: "Possível manipulação de preço detectada".to_string(),
                        severity: EventSeverity::High,
                    };

                    events.push(event);
                }
            }
        }

        Ok(events)
    }
}

/// Detector de liquidações suspeitas
pub struct SuspiciousLiquidationDetector;

impl SuspiciousLiquidationDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SpecializedDetector for SuspiciousLiquidationDetector {
    fn name(&self) -> &str {
        "SuspiciousLiquidationDetector"
    }

    async fn detect_events(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut events = Vec::new();

        // Liquidações suspeitas são detectadas por:
        // 1. Liquidações logo após manipulação de preço
        // 2. Liquidações de grandes quantidades
        // 3. Liquidador sendo o mesmo que manipulou o preço

        // Procura por padrões de liquidação
        for window in analysis.token_transfers.windows(3) {
            if let [transfer1, transfer2, transfer3] = window {
                // Verifica se há um padrão de: manipulação -> liquidação -> lucro
                if transfer1.amount > U256::from(100000) && // Grande transferência inicial
                    transfer2.from != transfer1.from && // Liquidação de outro usuário
                    transfer3.to == transfer1.from { // Lucro volta para o manipulador

                    let mut data = serde_json::Map::new();
                    data.insert("liquidator".to_string(), serde_json::Value::String(format!("{:?}", transfer1.from)));
                    data.insert("victim".to_string(), serde_json::Value::String(format!("{:?}", transfer2.from)));
                    data.insert("manipulation_amount".to_string(), serde_json::Value::String(transfer1.amount.to_string()));
                    data.insert("liquidation_amount".to_string(), serde_json::Value::String(transfer2.amount.to_string()));

                    let event = DetectedEvent {
                        event_type: "suspicious_liquidation".to_string(),
                        confidence: 0.75,
                        addresses: vec![transfer1.from, transfer2.from, transfer1.token_address],
                        data: serde_json::Value::Object(data),
                        description: "Liquidação suspeita detectada".to_string(),
                        severity: EventSeverity::High,
                    };

                    events.push(event);
                }
            }
        }

        Ok(events)
    }
}

/// Gerenciador de detectores especializados
pub struct DetectorManager {
    detectors: Vec<Box<dyn SpecializedDetector>>,
}

impl DetectorManager {
    /// Cria um novo gerenciador com detectores padrão
    pub fn new() -> Self {
        let mut detectors: Vec<Box<dyn SpecializedDetector>> = Vec::new();

        detectors.push(Box::new(SandwichAttackDetector::new()));
        detectors.push(Box::new(FrontrunningDetector::new()));
        detectors.push(Box::new(ReentrancyDetector::new()));
        detectors.push(Box::new(PriceManipulationDetector::new()));
        detectors.push(Box::new(SuspiciousLiquidationDetector::new()));

        Self { detectors }
    }

    /// Adiciona um detector personalizado
    pub fn add_detector(&mut self, detector: Box<dyn SpecializedDetector>) {
        self.detectors.push(detector);
    }

    /// Executa todos os detectores na análise
    pub async fn detect_all(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedEvent>, ()> {
        let mut all_events = Vec::new();

        for detector in &self.detectors {
            match detector.detect_events(analysis).await {
                Ok(mut events) => {
                    all_events.append(&mut events);
                },
                Err(e) => {
                    eprintln!("Erro no detector {}: {:?}", detector.name(), e);
                }
            }
        }

        Ok(all_events)
    }

    /// Obtém lista de detectores disponíveis
    pub fn available_detectors(&self) -> Vec<&str> {
        self.detectors.iter().map(|d| d.name()).collect()
    }
}

impl Default for DetectorManager {
    fn default() -> Self {
        Self::new()
    }
}