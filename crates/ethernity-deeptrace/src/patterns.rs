/*!
 * Ethernity DeepTrace - Patterns
 *
 * Detectores de padrões em transações blockchain
 */

use crate::{analyzer::TraceAnalysisResult, DetectedPattern, PatternType};
use async_trait::async_trait;
use ethereum_types::{Address, U256};
use std::collections::HashMap;

/// Trait para detectores de padrões
#[async_trait]
pub trait PatternDetector: Send + Sync {
    /// Tipo de padrão que este detector identifica
    fn pattern_type(&self) -> PatternType;

    /// Detecta padrões na análise de trace
    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()>;

    /// Confiança mínima para reportar um padrão
    fn min_confidence(&self) -> f64 {
        0.7
    }
}

/// Detector de padrões ERC20
pub struct Erc20PatternDetector;

impl Erc20PatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for Erc20PatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Erc20Creation
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        // Procura por criações de contratos ERC20
        for creation in &analysis.contract_creations {
            if matches!(creation.contract_type, crate::ContractType::Erc20Token) {
                let mut data = serde_json::Map::new();
                data.insert("contract_address".to_string(), serde_json::Value::String(format!("{:?}", creation.contract_address)));
                data.insert("creator".to_string(), serde_json::Value::String(format!("{:?}", creation.creator)));

                let pattern = DetectedPattern {
                    pattern_type: PatternType::Erc20Creation,
                    confidence: 0.9,
                    addresses: vec![creation.contract_address, creation.creator],
                    data: serde_json::Value::Object(data),
                    description: "Criação de token ERC20 detectada".to_string(),
                };

                patterns.push(pattern);
            }
        }

        Ok(patterns)
    }
}

/// Detector de padrões ERC721
pub struct Erc721PatternDetector;

impl Erc721PatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for Erc721PatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Erc721Creation
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        // Procura por criações de contratos ERC721
        for creation in &analysis.contract_creations {
            if matches!(creation.contract_type, crate::ContractType::Erc721Token) {
                let mut data = serde_json::Map::new();
                data.insert("contract_address".to_string(), serde_json::Value::String(format!("{:?}", creation.contract_address)));
                data.insert("creator".to_string(), serde_json::Value::String(format!("{:?}", creation.creator)));

                let pattern = DetectedPattern {
                    pattern_type: PatternType::Erc721Creation,
                    confidence: 0.9,
                    addresses: vec![creation.contract_address, creation.creator],
                    data: serde_json::Value::Object(data),
                    description: "Criação de token ERC721 (NFT) detectada".to_string(),
                };

                patterns.push(pattern);
            }
        }

        Ok(patterns)
    }
}

/// Detector de padrões DEX
pub struct DexPatternDetector;

impl DexPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for DexPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::TokenSwap
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        // Procura por padrões de swap (múltiplas transferências de tokens diferentes)
        if analysis.token_transfers.len() >= 2 {
            let mut token_groups: HashMap<Address, Vec<&crate::TokenTransfer>> = HashMap::new();

            // Agrupa transferências por token
            for transfer in &analysis.token_transfers {
                token_groups.entry(transfer.token_address).or_default().push(transfer);
            }

            // Se há transferências de pelo menos 2 tokens diferentes, pode ser um swap
            if token_groups.len() >= 2 {
                let mut confidence = 0.6;
                let mut addresses = Vec::new();
                let mut data = serde_json::Map::new();

                // Analisa padrão de transferências
                let tokens: Vec<_> = token_groups.keys().collect();
                for (i, &token) in tokens.iter().enumerate() {
                    addresses.push(*token);
                    data.insert(format!("token_{}", i), serde_json::Value::String(format!("{:?}", token)));
                }

                // Aumenta confiança se há transferências bidirecionais
                let mut has_bidirectional = false;
                for transfers in token_groups.values() {
                    if transfers.len() > 1 {
                        has_bidirectional = true;
                        break;
                    }
                }

                if has_bidirectional {
                    confidence += 0.2;
                }

                if confidence >= self.min_confidence() {
                    let pattern = DetectedPattern {
                        pattern_type: PatternType::TokenSwap,
                        confidence,
                        addresses,
                        data: serde_json::Value::Object(data),
                        description: "Padrão de swap de tokens detectado".to_string(),
                    };

                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }
}

/// Detector de padrões de lending
pub struct LendingPatternDetector;

impl LendingPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for LendingPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Liquidity
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        // Procura por padrões de lending (depósitos/retiradas de liquidez)
        // Analisa se há transferências grandes seguidas de transferências menores (juros)
        for window in analysis.token_transfers.windows(2) {
            if let [transfer1, transfer2] = window {
                // Verifica se são do mesmo token mas direções opostas
                if transfer1.token_address == transfer2.token_address &&
                    transfer1.from == transfer2.to &&
                    transfer1.to == transfer2.from {

                    let ratio = if transfer2.amount > U256::zero() {
                        transfer1.amount.as_u128() as f64 / transfer2.amount.as_u128() as f64
                    } else {
                        0.0
                    };

                    // Se uma transferência é significativamente maior, pode ser lending
                    if ratio > 1.1 && ratio < 100.0 {
                        let mut data = serde_json::Map::new();
                        data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", transfer1.token_address)));
                        data.insert("principal".to_string(), serde_json::Value::String(transfer1.amount.to_string()));
                        data.insert("repayment".to_string(), serde_json::Value::String(transfer2.amount.to_string()));
                        data.insert("interest_ratio".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(ratio - 1.0).unwrap()));

                        let pattern = DetectedPattern {
                            pattern_type: PatternType::Liquidity,
                            confidence: 0.75,
                            addresses: vec![transfer1.token_address, transfer1.from, transfer1.to],
                            data: serde_json::Value::Object(data),
                            description: "Padrão de empréstimo/liquidez detectado".to_string(),
                        };

                        patterns.push(pattern);
                    }
                }
            }
        }

        Ok(patterns)
    }
}

/// Detector de padrões de flash loan
pub struct FlashLoanPatternDetector;

impl FlashLoanPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for FlashLoanPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::FlashLoan
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        // Flash loans são caracterizados por:
        // 1. Empréstimo grande no início
        // 2. Múltiplas operações no meio
        // 3. Repagamento no final da mesma transação

        if analysis.token_transfers.len() >= 3 {
            // Procura por padrão: empréstimo -> operações -> repagamento
            let first_transfer = &analysis.token_transfers[0];
            let last_transfer = analysis.token_transfers.last().unwrap();

            // Verifica se primeira e última transferência são do mesmo token
            if first_transfer.token_address == last_transfer.token_address {
                // Verifica se há repagamento (direção oposta)
                if first_transfer.to == last_transfer.from &&
                    first_transfer.from == last_transfer.to {

                    // Verifica se o valor do repagamento é maior (incluindo taxa)
                    if last_transfer.amount >= first_transfer.amount {
                        let fee_ratio = if first_transfer.amount > U256::zero() {
                            (last_transfer.amount - first_transfer.amount).as_u128() as f64 /
                                first_transfer.amount.as_u128() as f64
                        } else {
                            0.0
                        };

                        // Taxa típica de flash loan é 0.05% a 0.3%
                        if fee_ratio >= 0.0005 && fee_ratio <= 0.01 {
                            let mut data = serde_json::Map::new();
                            data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", first_transfer.token_address)));
                            data.insert("amount".to_string(), serde_json::Value::String(first_transfer.amount.to_string()));
                            data.insert("fee_ratio".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(fee_ratio).unwrap()));
                            data.insert("intermediate_operations".to_string(), serde_json::Value::Number(serde_json::Number::from(analysis.token_transfers.len() - 2)));

                            let pattern = DetectedPattern {
                                pattern_type: PatternType::FlashLoan,
                                confidence: 0.85,
                                addresses: vec![first_transfer.token_address, first_transfer.from, first_transfer.to],
                                data: serde_json::Value::Object(data),
                                description: "Flash loan detectado".to_string(),
                            };

                            patterns.push(pattern);
                        }
                    }
                }
            }
        }

        Ok(patterns)
    }
}

/// Detector de padrões MEV
pub struct MevPatternDetector;

impl MevPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for MevPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Arbitrage
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        // MEV patterns incluem arbitragem, frontrunning, etc.
        // Procura por padrões de arbitragem: compra em um lugar, vende em outro

        if analysis.token_transfers.len() >= 4 {
            // Agrupa transferências por token
            let mut token_flows: HashMap<Address, Vec<(Address, Address, U256)>> = HashMap::new();

            for transfer in &analysis.token_transfers {
                token_flows.entry(transfer.token_address)
                    .or_default()
                    .push((transfer.from, transfer.to, transfer.amount));
            }

            // Procura por tokens que têm fluxo circular (arbitragem)
            for (token, flows) in token_flows {
                if flows.len() >= 2 {
                    // Verifica se há compra e venda do mesmo token
                    let mut net_flow: HashMap<Address, i128> = HashMap::new();

                    for (from, to, amount) in flows {
                        let amount_i128 = amount.as_u128() as i128;
                        *net_flow.entry(from).or_default() -= amount_i128;
                        *net_flow.entry(to).or_default() += amount_i128;
                    }

                    // Se alguém tem fluxo líquido positivo significativo, pode ser arbitragem
                    for (address, net) in net_flow {
                        if net > 0 && net as u128 > 1000 { // Threshold mínimo
                            let mut data = serde_json::Map::new();
                            data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", token)));
                            data.insert("arbitrageur".to_string(), serde_json::Value::String(format!("{:?}", address)));
                            data.insert("profit".to_string(), serde_json::Value::String(net.to_string()));

                            let pattern = DetectedPattern {
                                pattern_type: PatternType::Arbitrage,
                                confidence: 0.8,
                                addresses: vec![token, address],
                                data: serde_json::Value::Object(data),
                                description: "Padrão de arbitragem MEV detectado".to_string(),
                            };

                            patterns.push(pattern);
                        }
                    }
                }
            }
        }

        Ok(patterns)
    }
}

/// Detector de padrões de rug pull
pub struct RugPullPatternDetector;

impl RugPullPatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for RugPullPatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::RugPull
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        // Rug pulls são caracterizados por:
        // 1. Criação de token
        // 2. Remoção massiva de liquidez pelo criador
        // 3. Transferências grandes para o criador

        for creation in &analysis.contract_creations {
            if matches!(creation.contract_type, crate::ContractType::Erc20Token) {
                // Procura por transferências grandes do token criado para o criador
                let mut suspicious_transfers = Vec::new();
                let mut total_to_creator = U256::zero();

                for transfer in &analysis.token_transfers {
                    if transfer.token_address == creation.contract_address &&
                        transfer.to == creation.creator {
                        suspicious_transfers.push(transfer);
                        total_to_creator += transfer.amount;
                    }
                }

                // Se há transferências significativas para o criador
                if !suspicious_transfers.is_empty() && total_to_creator > U256::from(1000000) {
                    let mut data = serde_json::Map::new();
                    data.insert("token".to_string(), serde_json::Value::String(format!("{:?}", creation.contract_address)));
                    data.insert("creator".to_string(), serde_json::Value::String(format!("{:?}", creation.creator)));
                    data.insert("suspicious_amount".to_string(), serde_json::Value::String(total_to_creator.to_string()));
                    data.insert("transfer_count".to_string(), serde_json::Value::Number(serde_json::Number::from(suspicious_transfers.len())));

                    let confidence = if suspicious_transfers.len() > 3 { 0.9 } else { 0.7 };

                    let pattern = DetectedPattern {
                        pattern_type: PatternType::RugPull,
                        confidence,
                        addresses: vec![creation.contract_address, creation.creator],
                        data: serde_json::Value::Object(data),
                        description: "Possível rug pull detectado".to_string(),
                    };

                    patterns.push(pattern);
                }
            }
        }

        Ok(patterns)
    }
}

/// Detector de padrões de governança
pub struct GovernancePatternDetector;

impl GovernancePatternDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PatternDetector for GovernancePatternDetector {
    fn pattern_type(&self) -> PatternType {
        PatternType::Governance
    }

    async fn detect(&self, analysis: &TraceAnalysisResult) -> Result<Vec<DetectedPattern>, ()> {
        let mut patterns = Vec::new();

        // Padrões de governança são detectados por:
        // 1. Chamadas para funções de governança conhecidas
        // 2. Múltiplas interações com contratos de governança
        // 3. Transferências de tokens de governança

        // Assinaturas de funções de governança conhecidas
        let governance_signatures = [
            &[0xda, 0x35, 0xc6, 0x64], // propose(...)
            &[0x15, 0x37, 0x3e, 0x3d], // vote(...)
            &[0xfe, 0x0d, 0x94, 0xc1], // execute(...)
            &[0x40, 0xe5, 0x8e, 0xe5], // queue(...)
        ];

        // Percorre a árvore de chamadas procurando por assinaturas de governança
        analysis.call_tree.traverse_preorder(|node| {
            if !node.input.is_empty() && node.input.len() >= 4 {
                let function_sig = &node.input[0..4];

                for &gov_sig in &governance_signatures {
                    if function_sig == gov_sig {
                        let mut data = serde_json::Map::new();
                        data.insert("contract".to_string(), serde_json::Value::String(format!("{:?}", node.to.unwrap_or_else(|| Address::zero()))));
                        data.insert("caller".to_string(), serde_json::Value::String(format!("{:?}", node.from)));
                        data.insert("function_signature".to_string(), serde_json::Value::String(hex::encode(function_sig)));

                        let pattern = DetectedPattern {
                            pattern_type: PatternType::Governance,
                            confidence: 0.85,
                            addresses: vec![node.from, node.to.unwrap_or_else(|| Address::zero())],
                            data: serde_json::Value::Object(data),
                            description: "Atividade de governança detectada".to_string(),
                        };

                        patterns.push(pattern);
                        break;
                    }
                }
            }
        });

        Ok(patterns)
    }
}