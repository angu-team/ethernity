/*!
 * Ethernity DeepTrace - Utils
 *
 * Utilitários para análise de traces
 */

use ethereum_types::{Address, H256, U256};
use std::collections::HashMap;

/// Utilitários para análise de bytecode
pub struct BytecodeAnalyzer;

impl BytecodeAnalyzer {
    /// Extrai seletores de função do bytecode
    pub fn extract_function_selectors(bytecode: &[u8]) -> Vec<[u8; 4]> {
        let mut selectors = Vec::new();

        // Procura por padrões de PUSH4 seguido de EQ (comparação de seletor)
        for i in 0..bytecode.len().saturating_sub(6) {
            if bytecode[i] == 0x63 { // PUSH4
                let selector = [
                    bytecode[i + 1],
                    bytecode[i + 2],
                    bytecode[i + 3],
                    bytecode[i + 4],
                ];
                selectors.push(selector);
            }
        }

        selectors
    }

    /// Verifica se o bytecode contém um padrão específico
    pub fn contains_pattern(bytecode: &[u8], pattern: &[u8]) -> bool {
        bytecode.windows(pattern.len()).any(|window| window == pattern)
    }

    /// Conta a ocorrência de um opcode específico
    pub fn count_opcode(bytecode: &[u8], opcode: u8) -> usize {
        bytecode.iter().filter(|&&b| b == opcode).count()
    }

    /// Analisa a complexidade do bytecode
    pub fn analyze_complexity(bytecode: &[u8]) -> BytecodeComplexity {
        let mut complexity = BytecodeComplexity::default();

        for &byte in bytecode {
            match byte {
                0x00..=0x0f => complexity.arithmetic_ops += 1,
                0x10..=0x1f => complexity.comparison_ops += 1,
                0x20..=0x2f => complexity.crypto_ops += 1,
                0x30..=0x3f => complexity.env_ops += 1,
                0x40..=0x4f => complexity.block_ops += 1,
                0x50..=0x5f => complexity.storage_ops += 1,
                0x60..=0x6f => complexity.push_ops += 1,
                0x80..=0x8f => complexity.dup_ops += 1,
                0x90..=0x9f => complexity.swap_ops += 1,
                0xa0..=0xaf => complexity.log_ops += 1,
                0xf0..=0xff => complexity.system_ops += 1,
                _ => complexity.other_ops += 1,
            }
        }

        complexity.total_ops = bytecode.len();
        complexity
    }

    /// Detecta padrões de proxy
    pub fn detect_proxy_patterns(bytecode: &[u8]) -> Vec<ProxyPattern> {
        let mut patterns = Vec::new();

        // Padrão EIP-1167 (Minimal Proxy)
        let minimal_proxy_pattern = [
            0x36, 0x3d, 0x3d, 0x37, 0x3d, 0x3d, 0x3d, 0x36, 0x3d, 0x73
        ];
        if Self::contains_pattern(bytecode, &minimal_proxy_pattern) {
            patterns.push(ProxyPattern::MinimalProxy);
        }

        // Padrão de DELEGATECALL
        if bytecode.contains(&0xf4) { // DELEGATECALL opcode
            patterns.push(ProxyPattern::DelegateCall);
        }

        // Padrão de storage slot para implementação
        let implementation_slot_pattern = [
            0x7f, 0x36, 0x08, 0x94, 0xa1, 0x3b, 0xa1, 0xa3, 0x20, 0x6a
        ];
        if Self::contains_pattern(bytecode, &implementation_slot_pattern) {
            patterns.push(ProxyPattern::UpgradeableProxy);
        }

        patterns
    }
}

/// Complexidade do bytecode
#[derive(Debug, Default)]
pub struct BytecodeComplexity {
    pub total_ops: usize,
    pub arithmetic_ops: usize,
    pub comparison_ops: usize,
    pub crypto_ops: usize,
    pub env_ops: usize,
    pub block_ops: usize,
    pub storage_ops: usize,
    pub push_ops: usize,
    pub dup_ops: usize,
    pub swap_ops: usize,
    pub log_ops: usize,
    pub system_ops: usize,
    pub other_ops: usize,
}

impl BytecodeComplexity {
    /// Calcula um score de complexidade
    pub fn complexity_score(&self) -> f64 {
        let weighted_score =
            self.arithmetic_ops as f64 * 1.0 +
                self.comparison_ops as f64 * 1.2 +
                self.crypto_ops as f64 * 2.0 +
                self.storage_ops as f64 * 1.5 +
                self.system_ops as f64 * 3.0 +
                self.other_ops as f64 * 1.0;

        weighted_score / self.total_ops.max(1) as f64
    }
}

/// Padrões de proxy detectados
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyPattern {
    MinimalProxy,
    DelegateCall,
    UpgradeableProxy,
    BeaconProxy,
}

/// Analisador de fluxo de valor
pub struct ValueFlowAnalyzer;

impl ValueFlowAnalyzer {
    /// Analisa o fluxo de valor em uma transação
    pub fn analyze_value_flow(transfers: &[crate::TokenTransfer]) -> ValueFlowAnalysis {
        let mut analysis = ValueFlowAnalysis::default();

        // Agrupa transferências por endereço
        let mut address_flows: HashMap<Address, AddressFlow> = HashMap::new();

        for transfer in transfers {
            // Fluxo de saída
            let from_flow = address_flows.entry(transfer.from).or_default();
            from_flow.outgoing += transfer.amount;
            from_flow.tokens.insert(transfer.token_address);

            // Fluxo de entrada
            let to_flow = address_flows.entry(transfer.to).or_default();
            to_flow.incoming += transfer.amount;
            to_flow.tokens.insert(transfer.token_address);
        }

        // Calcula estatísticas
        for (address, flow) in address_flows {
            let net_flow = if flow.incoming >= flow.outgoing {
                flow.incoming - flow.outgoing
            } else {
                flow.outgoing - flow.incoming
            };

            if flow.incoming > flow.outgoing {
                analysis.net_receivers.push((address, net_flow));
            } else if flow.outgoing > flow.incoming {
                analysis.net_senders.push((address, net_flow));
            }

            analysis.total_addresses += 1;
            analysis.total_volume += flow.incoming + flow.outgoing;
        }

        // Ordena por volume
        analysis.net_receivers.sort_by(|a, b| b.1.cmp(&a.1));
        analysis.net_senders.sort_by(|a, b| b.1.cmp(&a.1));

        analysis
    }

    /// Detecta padrões suspeitos no fluxo de valor
    pub fn detect_suspicious_patterns(analysis: &ValueFlowAnalysis) -> Vec<SuspiciousPattern> {
        let mut patterns = Vec::new();

        // Concentração de valor
        if let Some((top_receiver, top_amount)) = analysis.net_receivers.first() {
            let mut total_received = U256::zero();
            for (_, amount) in &analysis.net_receivers {
                total_received += *amount;
            }

            let concentration = if total_received > U256::zero() {
                top_amount.as_u128() as f64 / total_received.as_u128() as f64
            } else {
                0.0
            };

            if concentration > 0.8 {
                patterns.push(SuspiciousPattern::HighConcentration {
                    address: *top_receiver,
                    concentration,
                });
            }
        }

        // Circular flow (possível wash trading)
        for (sender_addr, sender_amount) in &analysis.net_senders {
            for (receiver_addr, receiver_amount) in &analysis.net_receivers {
                if sender_addr == receiver_addr {
                    continue;
                }

                let ratio = sender_amount.as_u128() as f64 / receiver_amount.as_u128() as f64;
                if ratio > 0.9 && ratio < 1.1 {
                    patterns.push(SuspiciousPattern::CircularFlow {
                        address1: *sender_addr,
                        address2: *receiver_addr,
                        amount: *sender_amount,
                    });
                }
            }
        }

        patterns
    }
}

/// Análise de fluxo de valor
#[derive(Debug, Default)]
pub struct ValueFlowAnalysis {
    pub net_receivers: Vec<(Address, U256)>,
    pub net_senders: Vec<(Address, U256)>,
    pub total_addresses: usize,
    pub total_volume: U256,
}

/// Fluxo de um endereço específico
#[derive(Debug, Default)]
struct AddressFlow {
    incoming: U256,
    outgoing: U256,
    tokens: std::collections::HashSet<Address>,
}

/// Padrões suspeitos detectados
#[derive(Debug, Clone)]
pub enum SuspiciousPattern {
    HighConcentration {
        address: Address,
        concentration: f64,
    },
    CircularFlow {
        address1: Address,
        address2: Address,
        amount: U256,
    },
    RapidTransfers {
        addresses: Vec<Address>,
        frequency: f64,
    },
}

/// Utilitários para análise de gas
pub struct GasAnalyzer;

impl GasAnalyzer {
    /// Analisa o uso de gas em uma transação
    pub fn analyze_gas_usage(execution_path: &[crate::ExecutionStep]) -> GasAnalysis {
        let mut analysis = GasAnalysis::default();

        for step in execution_path {
            analysis.total_gas_used += step.gas_used;

            // Categoriza por tipo de operação
            match step.call_type {
                crate::trace::CallType::Call => analysis.call_gas += step.gas_used,
                crate::trace::CallType::StaticCall => analysis.static_call_gas += step.gas_used,
                crate::trace::CallType::DelegateCall => analysis.delegate_call_gas += step.gas_used,
                crate::trace::CallType::Create => analysis.create_gas += step.gas_used,
                crate::trace::CallType::Create2 => analysis.create2_gas += step.gas_used,
                _ => {} // Outros tipos
            }

            // Detecta operações caras
            if step.gas_used > U256::from(100000) {
                analysis.expensive_operations.push(ExpensiveOperation {
                    call_type: step.call_type,
                    from: step.from,
                    to: step.to,
                    gas_used: step.gas_used,
                    depth: step.depth,
                });
            }
        }

        analysis.operation_count = execution_path.len();
        analysis
    }

    /// Detecta padrões anômalos de gas
    pub fn detect_gas_anomalies(analysis: &GasAnalysis) -> Vec<GasAnomaly> {
        let mut anomalies = Vec::new();

        // Gas usage muito alto
        if analysis.total_gas_used > U256::from(10_000_000) {
            anomalies.push(GasAnomaly::ExcessiveGasUsage {
                total_gas: analysis.total_gas_used,
            });
        }

        // Muitas operações caras
        if analysis.expensive_operations.len() > 10 {
            anomalies.push(GasAnomaly::TooManyExpensiveOperations {
                count: analysis.expensive_operations.len(),
            });
        }

        // Proporção anômala de delegate calls
        let delegate_ratio = if analysis.total_gas_used > U256::zero() {
            analysis.delegate_call_gas.as_u128() as f64 / analysis.total_gas_used.as_u128() as f64
        } else {
            0.0
        };

        if delegate_ratio > 0.5 {
            anomalies.push(GasAnomaly::HighDelegateCallRatio {
                ratio: delegate_ratio,
            });
        }

        anomalies
    }
}

/// Análise de uso de gas
#[derive(Debug, Default)]
pub struct GasAnalysis {
    pub total_gas_used: U256,
    pub call_gas: U256,
    pub static_call_gas: U256,
    pub delegate_call_gas: U256,
    pub create_gas: U256,
    pub create2_gas: U256,
    pub operation_count: usize,
    pub expensive_operations: Vec<ExpensiveOperation>,
}

/// Operação cara em termos de gas
#[derive(Debug, Clone)]
pub struct ExpensiveOperation {
    pub call_type: crate::trace::CallType,
    pub from: Address,
    pub to: Address,
    pub gas_used: U256,
    pub depth: usize,
}

/// Anomalias de gas detectadas
#[derive(Debug, Clone)]
pub enum GasAnomaly {
    ExcessiveGasUsage {
        total_gas: U256,
    },
    TooManyExpensiveOperations {
        count: usize,
    },
    HighDelegateCallRatio {
        ratio: f64,
    },
    UnusualGasPattern {
        description: String,
    },
}

/// Utilitários para formatação e exibição
pub struct DisplayUtils;

impl DisplayUtils {
    /// Formata um endereço para exibição
    pub fn format_address(address: &Address) -> String {
        format!("0x{:x}", address)
    }

    /// Formata um valor U256 com unidades apropriadas
    pub fn format_amount(amount: &U256, decimals: u8) -> String {
        ethernity_core::utils::format_token_amount(amount, decimals)
    }

    /// Formata gas para exibição
    pub fn format_gas(gas: &U256) -> String {
        let gas_value = gas.as_u128();

        if gas_value >= 1_000_000 {
            format!("{:.2}M", gas_value as f64 / 1_000_000.0)
        } else if gas_value >= 1_000 {
            format!("{:.2}K", gas_value as f64 / 1_000.0)
        } else {
            gas_value.to_string()
        }
    }

    /// Cria um resumo textual da análise
    pub fn create_analysis_summary(analysis: &crate::TransactionAnalysis) -> String {
        let mut summary = String::new();

        // Converte H256 para Address para formatação
        let tx_hash_bytes: [u8; 32] = analysis.tx_hash.into();
        let tx_hash_addr = Address::from_slice(&tx_hash_bytes[12..32]);

        summary.push_str(&format!("Transação: {}\n", Self::format_address(&tx_hash_addr)));
        summary.push_str(&format!("Bloco: {}\n", analysis.block_number));
        summary.push_str(&format!("Status: {}\n", if analysis.status { "Sucesso" } else { "Falha" }));
        summary.push_str(&format!("Gas usado: {}\n", Self::format_gas(&analysis.gas_used)));
        summary.push_str(&format!("Transferências de token: {}\n", analysis.token_transfers.len()));
        summary.push_str(&format!("Contratos criados: {}\n", analysis.contract_creations.len()));
        summary.push_str(&format!("Padrões detectados: {}\n", analysis.detected_patterns.len()));
        summary.push_str(&format!("Profundidade máxima: {}\n", analysis.call_tree.max_depth()));

        if !analysis.detected_patterns.is_empty() {
            summary.push_str("\nPadrões detectados:\n");
            for pattern in &analysis.detected_patterns {
                summary.push_str(&format!("- {} (confiança: {:.2})\n", pattern.description, pattern.confidence));
            }
        }

        summary
    }
}

/// Utilitários para cache e otimização
pub struct CacheUtils;

impl CacheUtils {
    /// Calcula hash para cache de análise
    pub fn calculate_analysis_hash(tx_hash: &H256, config: &crate::TraceAnalysisConfig) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        tx_hash.hash(&mut hasher);
        config.max_depth.hash(&mut hasher);
        config.memory_limit.hash(&mut hasher);
        config.enable_cache.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Verifica se uma análise deve ser cacheada
    pub fn should_cache_analysis(analysis: &crate::TransactionAnalysis) -> bool {
        // Cacheia análises complexas ou com muitos padrões detectados
        analysis.call_tree.total_calls() > 10 ||
            analysis.detected_patterns.len() > 0 ||
            analysis.token_transfers.len() > 5
    }
}

/// Converte uma string hexadecimal em bytes.
pub fn decode_hex(data: &str) -> Vec<u8> {
    hex::decode(data.trim_start_matches("0x")).unwrap_or_default()
}

/// Converte uma string hexadecimal em um endereço Ethereum.
pub fn parse_address(hex_addr: &str) -> Address {
    let bytes = decode_hex(hex_addr);
    if bytes.len() >= 20 {
        Address::from_slice(&bytes[bytes.len() - 20..])
    } else {
        Address::zero()
    }
}

/// Converte uma string hexadecimal em U256.
pub fn parse_u256_hex(hex_val: &str) -> U256 {
    U256::from_str_radix(hex_val.trim_start_matches("0x"), 16).unwrap_or_else(|_| U256::zero())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CallNode, CallTree, CallType, TokenTransfer, TokenType, DetectedPattern, PatternType, ExecutionStep, TransactionAnalysis};
    use ethereum_types::{Address, H256};
    use serde_json::json;
    use chrono::Utc;

    #[test]
    fn test_bytecode_analysis_and_proxy_detection() {
        let mut code = vec![
            0x01, 0x11, 0x21, 0x31, 0x41, 0x51, 0x61, 0x80, 0x90, 0xa0, 0xf4, 0xf1, 0x00,
        ];
        code.extend_from_slice(&[0x36, 0x3d, 0x3d, 0x37, 0x3d, 0x3d, 0x3d, 0x36, 0x3d, 0x73]);
        code.extend_from_slice(&[0x7f, 0x36, 0x08, 0x94, 0xa1, 0x3b, 0xa1, 0xa3, 0x20, 0x6a]);

        let selectors = BytecodeAnalyzer::extract_function_selectors(&[0x63, 1, 2, 3, 4, 0, 0, 0x63, 5, 6, 7, 8, 0, 0]);
        assert_eq!(selectors, vec![[1,2,3,4],[5,6,7,8]]);
        assert!(BytecodeAnalyzer::contains_pattern(&[1,2,3,4,5], &[2,3]));
        assert_eq!(BytecodeAnalyzer::count_opcode(&[0x63,0,0x63], 0x63), 2);

        let complexity = BytecodeAnalyzer::analyze_complexity(&code);
        assert_eq!(complexity.total_ops, code.len());
        assert_eq!(complexity.arithmetic_ops, 3); // 0x01, 0x00 and 0x08
        assert_eq!(complexity.system_ops, 2); // 0xf4 and 0xf1

        assert!(BytecodeAnalyzer::detect_proxy_patterns(&code).contains(&ProxyPattern::MinimalProxy));
        assert!(BytecodeAnalyzer::detect_proxy_patterns(&code).contains(&ProxyPattern::DelegateCall));
        assert!(BytecodeAnalyzer::detect_proxy_patterns(&code).contains(&ProxyPattern::UpgradeableProxy));

        // complexity score should be calculable
        let _score = complexity.complexity_score();
    }

    #[test]
    fn test_value_flow_and_suspicious_patterns() {
        let addr = |n| Address::from_low_u64_be(n);
        let transfers = vec![
            TokenTransfer{token_type:TokenType::Erc20, token_address:addr(10), from:addr(1), to:addr(2), amount:U256::from(80u64), token_id:None, call_index:0},
            TokenTransfer{token_type:TokenType::Erc20, token_address:addr(10), from:addr(3), to:addr(2), amount:U256::from(10u64), token_id:None, call_index:0},
            TokenTransfer{token_type:TokenType::Erc20, token_address:addr(10), from:addr(2), to:addr(4), amount:U256::from(5u64), token_id:None, call_index:0},
        ];

        let analysis = ValueFlowAnalyzer::analyze_value_flow(&transfers);
        assert_eq!(analysis.total_addresses, 4);
        assert_eq!(analysis.net_receivers.first().unwrap().0, addr(2));

        let patterns = ValueFlowAnalyzer::detect_suspicious_patterns(&analysis);
        assert!(matches!(patterns[0], SuspiciousPattern::HighConcentration{..}));
        assert!(patterns.iter().any(|p| matches!(p, SuspiciousPattern::CircularFlow{..})));
    }

    #[test]
    fn test_gas_analysis_and_anomalies() {
        let addr = |n| Address::from_low_u64_be(n);
        let steps = vec![
            ExecutionStep{depth:0, call_type:crate::trace::CallType::Call, from:addr(1), to:addr(2), value:U256::zero(), input:vec![], output:vec![], gas_used:U256::from(50_000u64), error:None},
            ExecutionStep{depth:0, call_type:crate::trace::CallType::DelegateCall, from:addr(1), to:addr(3), value:U256::zero(), input:vec![], output:vec![], gas_used:U256::from(200_000u64), error:None},
            ExecutionStep{depth:0, call_type:crate::trace::CallType::Create, from:addr(1), to:addr(4), value:U256::zero(), input:vec![], output:vec![], gas_used:U256::from(1_000u64), error:None},
        ];

        let analysis = GasAnalyzer::analyze_gas_usage(&steps);
        assert_eq!(analysis.total_gas_used, U256::from(251_000u64));
        assert_eq!(analysis.expensive_operations.len(), 1);

        let heavy = GasAnalysis {
            total_gas_used: U256::from(20_000_000u64),
            call_gas: analysis.call_gas,
            static_call_gas: analysis.static_call_gas,
            delegate_call_gas: U256::from(12_000_000u64),
            create_gas: analysis.create_gas,
            create2_gas: analysis.create2_gas,
            operation_count: analysis.operation_count,
            expensive_operations: vec![analysis.expensive_operations[0].clone(); 11],
        };
        let anomalies = GasAnalyzer::detect_gas_anomalies(&heavy);
        assert_eq!(anomalies.len(), 3);
    }

    #[test]
    fn test_display_and_cache_utils_and_parsing() {
        let addr = Address::from_low_u64_be(1);
        assert_eq!(DisplayUtils::format_address(&addr), "0x0000000000000000000000000000000000000001");
        assert_eq!(DisplayUtils::format_gas(&U256::from(2_000_000u64)), "2.00M");
        assert_eq!(DisplayUtils::format_gas(&U256::from(2_000u64)), "2.00K");
        assert_eq!(DisplayUtils::format_gas(&U256::from(500u64)), "500");

        let root = CallNode{index:0, depth:0, call_type:CallType::Call, from:addr, to:Some(addr), value:U256::zero(), gas:U256::zero(), gas_used:U256::zero(), input:vec![], output:vec![], error:None, children:vec![]};
        let analysis = TransactionAnalysis{
            tx_hash:H256::from_low_u64_be(1),
            block_number:1,
            timestamp:Utc::now(),
            from:addr,
            to:Some(addr),
            value:U256::zero(),
            gas_used:U256::from(1234u64),
            status:true,
            call_tree:CallTree{root},
            token_transfers:vec![],
            contract_creations:vec![],
            detected_patterns:vec![DetectedPattern{pattern_type:PatternType::Unknown, confidence:0.9, addresses:vec![], data:json!(null), description:"p".into()}],
            execution_path:vec![]
        };
        let summary = DisplayUtils::create_analysis_summary(&analysis);
        assert!(summary.contains("Transação: 0x0000000000000000000000000000000000000001"));
        assert!(summary.contains("Padrões detectados:"));

        let config = crate::TraceAnalysisConfig::default();
        let h = CacheUtils::calculate_analysis_hash(&analysis.tx_hash, &config);
        assert!(!h.is_empty());
        assert!(CacheUtils::should_cache_analysis(&analysis));

        assert_eq!(decode_hex("0x0102"), vec![1u8,2u8]);
        assert_eq!(parse_address("0x0000000000000000000000000000000000000001"), addr);
        assert_eq!(parse_u256_hex("0xff"), U256::from(255u64));
    }
}