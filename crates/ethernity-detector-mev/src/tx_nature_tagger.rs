use ethernity_core::{error::Result, traits::RpcProvider, types::TransactionHash};
use ethereum_types::Address;
use lru::LruCache;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::num::NonZeroUsize;

/// Componentes que contribuem para o cálculo de confiança.
#[derive(Debug, Clone, Default)]
pub struct ConfidenceComponents {
    pub abi_match: f64,
    pub structure: f64,
    pub path: f64,
}

/// Resultado da inferência da natureza da transação.
#[derive(Debug, Clone, Default)]
pub struct TxNature {
    pub tx_hash: TransactionHash,
    pub tags: Vec<String>,
    pub token_paths: Vec<Address>,
    pub targets: Vec<Address>,
    pub confidence: f64,
    pub confidence_components: ConfidenceComponents,
    pub extracted_fallback: bool,
    pub ambiguous_execution_path: bool,
    pub reachable_via_dispatcher: bool,
    pub path_inference_failed: bool,
}

/// Tagger responsável por inferir a natureza de uma transação Ethereum.
pub struct TxNatureTagger<P> {
    provider: P,
    selectors: HashMap<[u8; 4], Vec<String>>,
    code_cache: Mutex<LruCache<Address, Vec<u8>>>,
}

impl<P> TxNatureTagger<P> {
    /// Cria uma nova instância de tagger.
    pub fn new(provider: P) -> Self {
        let mut selectors = HashMap::new();
        selectors.insert(
            [0x38, 0xed, 0x17, 0x39],
            vec!["swap-v2".to_string(), "router-call".to_string()],
        );
        selectors.insert(
            [0x18, 0xcb, 0xaf, 0x95],
            vec!["swap-v3".to_string(), "router-call".to_string()],
        );
        selectors.insert(
            [0xa9, 0x05, 0x9c, 0xbb],
            vec!["transfer".to_string(), "token-move".to_string()],
        );

        Self {
            provider,
            selectors,
            code_cache: Mutex::new(LruCache::new(NonZeroUsize::new(1024).unwrap())),
        }
    }
}

impl<P: RpcProvider + Send + Sync> TxNatureTagger<P> {
    /// Analisa uma transação e retorna sua provável natureza.
    pub async fn analyze(
        &self,
        to: Address,
        input: &[u8],
        tx_hash: TransactionHash,
    ) -> Result<TxNature> {
        // Seleciona os 4 primeiros bytes do calldata
        let selector = if input.len() >= 4 {
            Some([input[0], input[1], input[2], input[3]])
        } else {
            None
        };

        let mut result = TxNature {
            tx_hash,
            targets: vec![to],
            ..Default::default()
        };

        if let Some(sel) = selector {
            if let Some(tags) = self.selectors.get(&sel) {
                result.tags.extend(tags.clone());
                result.confidence_components.abi_match = 0.9;
            } else {
                result.confidence_components.abi_match = 0.1;
            }
        }

        // Recupera bytecode do contrato destino usando cache
        let code = self.get_code_cached(to).await?;

        // Heurística simples para delegação
        if code.iter().any(|&b| b == 0xf4u8) {
            result.tags.push("proxy-call".to_string());
            result.confidence_components.structure = 0.7;
        } else {
            result.confidence_components.structure = 0.5;
        }

        // Extração de possíveis endereços após o seletor
        if input.len() > 4 {
            let mut paths = Vec::new();
            for chunk in input[4..].chunks(32) {
                if chunk.len() == 32 {
                    let addr = Address::from_slice(&chunk[12..32]);
                    if addr != Address::zero() {
                        paths.push(addr);
                    }
                }
            }
            if !paths.is_empty() {
                result.token_paths = paths;
                result.extracted_fallback = true;
                result.confidence_components.path = 0.5;
            } else {
                result.path_inference_failed = true;
            }
        } else {
            result.path_inference_failed = true;
        }

        result.confidence = (
            result.confidence_components.abi_match
                + result.confidence_components.structure
                + result.confidence_components.path
        ) / 3.0;

        Ok(result)
    }

    async fn get_code_cached(&self, address: Address) -> Result<Vec<u8>> {
        {
            let mut cache = self.code_cache.lock();
            if let Some(code) = cache.get(&address) {
                return Ok(code.clone());
            }
        }

        let code = self.provider.get_code(address).await?;
        {
            let mut cache = self.code_cache.lock();
            cache.put(address, code.clone());
        }
        Ok(code)
    }
}

