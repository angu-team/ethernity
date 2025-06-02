# ethernity-core

**Tipos e utilitários compartilhados para a workspace Ethernity**

## Visão Geral

O `ethernity-core` é a fundação de toda a workspace Ethernity, fornecendo tipos, traits e utilitários compartilhados que garantem consistência e interoperabilidade entre todas as outras crates. Atua como a camada base que define contratos comuns e funcionalidades essenciais.

## Estrutura do Projeto

```
ethernity-core/
├── src/
│   ├── lib.rs          # Re-exportações principais e documentação do módulo
│   ├── types.rs        # Tipos comuns e enums
│   ├── traits.rs       # Traits compartilhadas
│   ├── utils.rs        # Utilitários e funções auxiliares
│   └── error.rs        # Sistema de erros unificado
├── Cargo.toml          # Dependências e metadados
└── README.md
```

## Dependências Principais

- **ethereum-types**: Tipos básicos do Ethereum (Address, H256, U256)
- **serde**: Serialização e deserialização
- **thiserror**: Definição de erros
- **async-trait**: Traits assíncronas
- **chrono**: Manipulação de data/hora
- **tiny-keccak**: Hashing Keccak-256
- **rlp**: Codificação RLP
- **secp256k1**: Criptografia de curva elíptica

---

## 📋 Módulo: types.rs

### Tipos Fundamentais

#### Aliases de Tipos
```rust
/// Hash de transação Ethereum
pub type TransactionHash = H256;
```

#### EventType - Tipos de Eventos Blockchain
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    Erc20Created,      // Criação de token ERC20
    TokenSwap,         // Troca de tokens
    LargeTransfer,     // Transferência grande
    Liquidation,       // Liquidação
    RugPullWarning,    // Aviso de rug pull
    MevActivity,       // Atividade MEV
    FlashLoan,         // Flash loan
    GovernanceEvent,   // Evento de governança
}
```

**Exemplo de uso:**
```rust
use ethernity_core::types::EventType;

let event_type = EventType::TokenSwap;
println!("Tipo do evento: {}", event_type); // "token_swap"

// Verificação de tipo
if matches!(event_type, EventType::TokenSwap | EventType::LargeTransfer) {
    println!("Evento relacionado a transferências");
}
```

#### TokenInfo - Informações de Token
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: Address,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub total_supply: Option<U256>,
}
```

**Exemplo de uso:**
```rust
let token = TokenInfo {
    address: Address::from_str("0xA0b86a33E6441e2e86D6DbD5b9c9a15a8b9a5f37")?,
    name: Some("USD Coin".to_string()),
    symbol: Some("USDC".to_string()),
    decimals: Some(6),
    total_supply: Some(U256::from_dec_str("45000000000000000")?),
};
```

#### DexProtocol - Protocolos DEX
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DexProtocol {
    UniswapV2,
    UniswapV3,
    SushiSwap,
    Curve,
    Balancer,
    OneInch,
    Paraswap,
    Unknown(String),
}
```

#### CreationType - Tipos de Criação de Contrato
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreationType {
    Create,    // CREATE opcode
    Create2,   // CREATE2 opcode
}
```

#### ContractPattern - Padrões de Contrato
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractPattern {
    Erc20Token,   // Token ERC20
    Proxy,        // Contrato proxy
    Diamond,      // Diamond pattern
    MinimalProxy, // EIP-1167 minimal proxy
    Factory,      // Factory pattern
    Multisig,     // Multisig wallet
    Unknown,      // Padrão desconhecido
}
```

#### AttackType - Tipos de Ataques
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    Reentrancy,         // Ataque de reentrância
    FlashLoanAttack,    // Ataque com flash loan
    PriceManipulation,  // Manipulação de preço
    GovernanceAttack,   // Ataque de governança
    RugPull,           // Rug pull
    Honeypot,          // Honeypot
    GasBomb,           // Gas bomb
    FrontRunning,      // Front running
    SandwichAttack,    // Sandwich attack
}
```

#### Severidade e Status
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionStatus {
    Success,
    Failure,
    Pending,
}
```

#### Identificadores Tipados
```rust
// Identificadores com type safety
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NotificationId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub String);
```

---

## 🔌 Módulo: traits.rs

### RpcProvider - Provedor RPC
Trait principal para interação com nodes Ethereum.

```rust
#[async_trait]
pub trait RpcProvider: Send + Sync {
    /// Obtém o trace de uma transação
    async fn get_transaction_trace(&self, tx_hash: TransactionHash) -> Result<Vec<u8>>;
    
    /// Obtém o recibo de uma transação
    async fn get_transaction_receipt(&self, tx_hash: TransactionHash) -> Result<Vec<u8>>;
    
    /// Obtém o código de um contrato
    async fn get_code(&self, address: Address) -> Result<Vec<u8>>;
    
    /// Chama um método de contrato
    async fn call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>>;

    /// Obtém o número do bloco atual
    async fn get_block_number(&self) -> Result<u64>;
}
```

**Implementação de exemplo:**
```rust
use ethernity_core::traits::RpcProvider;
use async_trait::async_trait;

struct CustomRpcProvider {
    endpoint: String,
}

#[async_trait]
impl RpcProvider for CustomRpcProvider {
    async fn get_transaction_trace(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        // Implementação customizada
        let trace = format!("{{\"result\":{{\"from\":\"0x...\",\"to\":\"0x...\"}}}");
        Ok(trace.into_bytes())
    }

    async fn get_transaction_receipt(&self, tx_hash: TransactionHash) -> Result<Vec<u8>> {
        // Implementação customizada
        Ok(vec![])
    }

    async fn get_code(&self, address: Address) -> Result<Vec<u8>> {
        // Implementação customizada
        Ok(vec![])
    }

    async fn call(&self, to: Address, data: Vec<u8>) -> Result<Vec<u8>> {
        // Implementação customizada
        Ok(vec![])
    }

    async fn get_block_number(&self) -> Result<u64> {
        // Implementação customizada
        Ok(12345678)
    }
}
```

### EventDetector - Detector de Eventos
```rust
#[async_trait]
pub trait EventDetector: Send + Sync {
    /// Tipo de evento detectado
    fn event_type(&self) -> EventType;
    
    /// Detecta eventos em uma transação
    async fn detect_events(&self, tx_hash: TransactionHash) -> Result<Vec<u8>>;
    
    /// Verifica se uma transação requer análise profunda
    fn requires_deep_trace(&self, tx_hash: TransactionHash) -> bool;
}
```

### EventNotifier - Notificador de Eventos
```rust
#[async_trait]
pub trait EventNotifier: Send + Sync {
    /// Envia uma notificação de evento
    async fn notify(&self, event_data: Vec<u8>) -> Result<()>;
    
    /// Verifica se o notificador está disponível
    async fn is_available(&self) -> bool;
}
```

---

## 🛠️ Módulo: utils.rs

### Conversões de Formato

#### Funções de Conversão Hexadecimal
```rust
/// Converte string hexadecimal para Address
pub fn hex_to_address(hex: &str) -> Option<Address>

/// Converte string hexadecimal para H256
pub fn hex_to_h256(hex: &str) -> Option<H256>

/// Converte string decimal para U256
pub fn decimal_to_u256(decimal: &str) -> Option<U256>
```

**Exemplos:**
```rust
use ethernity_core::utils::*;

// Conversão de endereço
let addr = hex_to_address("0x742d35Cc6634C0532925a3b8D400E1A1EA2fFc3D").unwrap();
let addr2 = hex_to_address("742d35Cc6634C0532925a3b8D400E1A1EA2fFc3D").unwrap(); // Sem 0x

// Conversão de hash
let hash = hex_to_h256("0x123abc...").unwrap();

// Conversão de valor
let amount = decimal_to_u256("1000000000000000000").unwrap(); // 1 ETH em wei
```

#### Funções de Formatação
```rust
/// Formata Address para exibição
pub fn format_address(address: &Address) -> String

/// Formata H256 para exibição  
pub fn format_h256(hash: &H256) -> String

/// Formata U256 para exibição
pub fn format_u256(value: &U256) -> String

/// Formata valor com decimais para exibição
pub fn format_token_amount(amount: &U256, decimals: u8) -> String
```

**Exemplos:**
```rust
// Formatação básica
let addr_str = format_address(&address); // "0x742d35Cc..."
let hash_str = format_h256(&hash);       // "0x123abc..."
let value_str = format_u256(&amount);    // "1000000000000000000"

// Formatação de token com decimais
let usdc_amount = U256::from(1_500_000); // 1.5 USDC em unidades base
let formatted = format_token_amount(&usdc_amount, 6); // "1.5"

let eth_amount = U256::from_dec_str("1500000000000000000")?; // 1.5 ETH
let formatted_eth = format_token_amount(&eth_amount, 18); // "1.5"
```

### Verificações Blockchain

#### Verificação de Contrato
```rust
/// Verifica se um endereço é um contrato
pub async fn is_contract<P: RpcProvider>(provider: &P, address: &Address) -> bool
```

**Exemplo:**
```rust
let rpc_provider = MyRpcProvider::new();
let address = hex_to_address("0x...").unwrap();

if is_contract(&rpc_provider, &address).await {
    println!("Endereço é um contrato");
} else {
    println!("Endereço é uma EOA (conta externa)");
}
```

#### Verificação ERC20
```rust
/// Verifica se um endereço é um contrato ERC20
pub async fn is_erc20<P: RpcProvider>(provider: &P, address: &Address) -> bool
```

**Exemplo:**
```rust
if is_erc20(&rpc_provider, &address).await {
    println!("Contrato implementa ERC20");
    
    // Pode chamar funções ERC20
    let balance_selector = [0x70, 0xa0, 0x82, 0x31]; // balanceOf(address)
    let mut call_data = balance_selector.to_vec();
    call_data.extend_from_slice(&[0; 32]); // address padded
    
    let result = rpc_provider.call(address, call_data).await?;
}
```

### Cálculos de Endereços

#### CREATE Address Calculation
```rust
/// Calcula o endereço de um contrato criado via CREATE
pub fn calculate_create_address(sender: &Address, nonce: u64) -> Address
```

**Exemplo:**
```rust
let deployer = hex_to_address("0x742d35Cc6634C0532925a3b8D400E1A1EA2fFc3D").unwrap();
let nonce = 42;

let contract_address = calculate_create_address(&deployer, nonce);
println!("Contrato será criado em: {}", format_address(&contract_address));
```

#### CREATE2 Address Calculation
```rust
/// Calcula o endereço de um contrato criado via CREATE2
pub fn calculate_create2_address(
    sender: &Address, 
    salt: &H256, 
    init_code_hash: &H256
) -> Address
```

**Exemplo:**
```rust
let deployer = hex_to_address("0x742d35Cc6634C0532925a3b8D400E1A1EA2fFc3D").unwrap();
let salt = H256::from_str("0x123...")?;
let init_code_hash = H256::from_str("0x456...")?;

let contract_address = calculate_create2_address(&deployer, &salt, &init_code_hash);
println!("Contrato CREATE2 será criado em: {}", format_address(&contract_address));
```

### Funções Criptográficas

#### Keccak-256 Hashing
```rust
/// Calcula o hash Keccak-256 de dados
pub fn keccak256(data: &[u8]) -> [u8; 32]
```

**Exemplo:**
```rust
let data = b"Hello, Ethereum!";
let hash = keccak256(data);
println!("Hash: 0x{}", hex::encode(hash));

// Hash de função
let function_signature = b"transfer(address,uint256)";
let function_selector = &keccak256(function_signature)[0..4];
println!("Seletor: 0x{}", hex::encode(function_selector)); // 0xa9059cbb
```

#### Ethereum Signed Message
```rust
/// Calcula o hash de uma mensagem no formato Ethereum Signed Message
pub fn eth_message_hash(message: &[u8]) -> H256
```

**Exemplo:**
```rust
let message = b"Please sign this message";
let message_hash = eth_message_hash(message);

// O hash inclui o prefixo "\x19Ethereum Signed Message:\n{length}"
println!("Hash da mensagem: {}", format_h256(&message_hash));
```

#### Recuperação de Assinatura
```rust
/// Recupera o endereço que assinou uma mensagem
pub fn recover_signer(message_hash: &H256, signature: &[u8]) -> Option<Address>
```

**Exemplo:**
```rust
let message = b"Verification message";
let message_hash = eth_message_hash(message);

// Assinatura de 65 bytes (r + s + v)
let signature: Vec<u8> = vec![/* 65 bytes de assinatura */];

if let Some(signer) = recover_signer(&message_hash, &signature) {
    println!("Mensagem assinada por: {}", format_address(&signer));
} else {
    println!("Falha ao recuperar assinante");
}
```

---

## ❗ Módulo: error.rs

### Sistema de Erros Unificado

```rust
#[derive(Error, Debug)]
pub enum Error {
    /// Erro de comunicação com o node Ethereum
    #[error("Erro de RPC: {0}")]
    RpcError(String),
    
    /// Erro de decodificação de dados
    #[error("Erro de decodificação: {0}")]
    DecodeError(String),
    
    /// Erro de codificação de dados
    #[error("Erro de codificação: {0}")]
    EncodeError(String),
    
    /// Erro de validação
    #[error("Erro de validação: {0}")]
    ValidationError(String),
    
    /// Erro de timeout
    #[error("Timeout: {0}")]
    TimeoutError(String),
    
    /// Recurso não encontrado
    #[error("Não encontrado: {0}")]
    NotFound(String),
    
    /// Erro genérico
    #[error("{0}")]
    Other(String),
}

/// Tipo de resultado usado em toda a biblioteca
pub type Result<T> = std::result::Result<T, Error>;
```

### Uso do Sistema de Erros

```rust
use ethernity_core::{Error, Result};

fn parse_address(input: &str) -> Result<Address> {
    Address::from_str(input.trim_start_matches("0x"))
        .map_err(|e| Error::DecodeError(format!("Endereço inválido '{}': {}", input, e)))
}

async fn get_balance(provider: &impl RpcProvider, address: Address) -> Result<U256> {
    let call_data = vec![0x70, 0xa0, 0x82, 0x31]; // balanceOf selector
    
    let result = provider.call(address, call_data).await
        .map_err(|e| Error::RpcError(format!("Falha ao obter saldo: {}", e)))?;
    
    if result.len() < 32 {
        return Err(Error::DecodeError("Resposta muito curta para U256".to_string()));
    }
    
    Ok(U256::from_big_endian(&result[0..32]))
}

// Uso com pattern matching
match get_balance(&provider, address).await {
    Ok(balance) => println!("Saldo: {}", balance),
    Err(Error::RpcError(msg)) => eprintln!("Erro de conectividade: {}", msg),
    Err(Error::DecodeError(msg)) => eprintln!("Erro de formato: {}", msg),
    Err(e) => eprintln!("Erro geral: {}", e),
}
```

---

## 🚀 Exemplos Avançados

### Verificação Completa de Token ERC20
```rust
use ethernity_core::{types::*, utils::*, Error, Result};

async fn analyze_token(
    provider: &impl RpcProvider, 
    address: Address
) -> Result<TokenInfo> {
    // Verificar se é contrato
    if !is_contract(provider, &address).await {
        return Err(Error::ValidationError("Endereço não é um contrato".to_string()));
    }
    
    // Verificar se é ERC20
    if !is_erc20(provider, &address).await {
        return Err(Error::ValidationError("Contrato não implementa ERC20".to_string()));
    }
    
    // Obter informações do token
    let mut token_info = TokenInfo {
        address,
        name: None,
        symbol: None,
        decimals: None,
        total_supply: None,
    };
    
    // name()
    let name_selector = keccak256(b"name()")[0..4].to_vec();
    if let Ok(result) = provider.call(address, name_selector).await {
        if result.len() >= 64 {
            // Decodificar string ABI
            let offset = U256::from_big_endian(&result[0..32]).as_usize();
            let length = U256::from_big_endian(&result[32..64]).as_usize();
            if offset + length <= result.len() && length <= 1000 {
                if let Ok(name_str) = String::from_utf8(result[offset..offset+length].to_vec()) {
                    token_info.name = Some(name_str);
                }
            }
        }
    }
    
    // symbol()
    let symbol_selector = keccak256(b"symbol()")[0..4].to_vec();
    if let Ok(result) = provider.call(address, symbol_selector).await {
        // Similar ao name()
        // ... implementação
    }
    
    // decimals()
    let decimals_selector = keccak256(b"decimals()")[0..4].to_vec();
    if let Ok(result) = provider.call(address, decimals_selector).await {
        if result.len() >= 32 {
            let decimals = U256::from_big_endian(&result[0..32]);
            if decimals <= U256::from(255) {
                token_info.decimals = Some(decimals.as_u32() as u8);
            }
        }
    }
    
    // totalSupply()
    let total_supply_selector = keccak256(b"totalSupply()")[0..4].to_vec();
    if let Ok(result) = provider.call(address, total_supply_selector).await {
        if result.len() >= 32 {
            token_info.total_supply = Some(U256::from_big_endian(&result[0..32]));
        }
    }
    
    Ok(token_info)
}
```

### Validação de Assinatura Completa
```rust
async fn verify_signature(
    message: &str,
    signature_hex: &str,
    expected_signer: Address
) -> Result<bool> {
    // Decodificar assinatura
    let signature = hex::decode(signature_hex.trim_start_matches("0x"))
        .map_err(|e| Error::DecodeError(format!("Assinatura inválida: {}", e)))?;
    
    if signature.len() != 65 {
        return Err(Error::ValidationError("Assinatura deve ter 65 bytes".to_string()));
    }
    
    // Calcular hash da mensagem
    let message_hash = eth_message_hash(message.as_bytes());
    
    // Recuperar assinante
    let recovered_signer = recover_signer(&message_hash, &signature)
        .ok_or_else(|| Error::DecodeError("Falha ao recuperar assinante".to_string()))?;
    
    Ok(recovered_signer == expected_signer)
}

// Uso
let is_valid = verify_signature(
    "Please sign this message",
    "0x1234567890abcdef...", // 130 caracteres hex (65 bytes)
    expected_signer_address
).await?;
```

---

## 🧪 Testes

### Executar Testes
```bash
cd crates/ethernity-core
cargo test
```

### Testes de Unidade Importantes
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_address_conversion() {
        let addr_str = "0x742d35Cc6634C0532925a3b8D400E1A1EA2fFc3D";
        let addr = hex_to_address(addr_str).unwrap();
        assert_eq!(format_address(&addr), addr_str.to_lowercase());
    }
    
    #[test]
    fn test_keccak256() {
        let data = b"";
        let hash = keccak256(data);
        let expected = "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";
        assert_eq!(hex::encode(hash), expected);
    }
    
    #[test]
    fn test_create_address_calculation() {
        // Exemplo conhecido do Ethereum
        let sender = hex_to_address("0x6ac7ea33f8831ea9dcc53393aaa88b25a785dbf0").unwrap();
        let nonce = 1;
        let expected = hex_to_address("0x343c43a37d37dff08ae8c4a11544c718abb4fcf8").unwrap();
        
        let calculated = calculate_create_address(&sender, nonce);
        assert_eq!(calculated, expected);
    }
}
```

## 📚 Recursos Adicionais

- [Documentação de ethereum-types](https://docs.rs/ethereum-types/)
- [Especificação EIP-1167 (Minimal Proxy)](https://eips.ethereum.org/EIPS/eip-1167)
- [Ethereum RLP Encoding](https://ethereum.org/en/developers/docs/data-structures-and-encoding/rlp/)
- [EIP-191 (Signed Data Standard)](https://eips.ethereum.org/EIPS/eip-191)
