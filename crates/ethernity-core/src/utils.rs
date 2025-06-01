/*!
 * Ethernity Utils
 * 
 * Utilitários comuns usados em toda a workspace Ethernity
 */

use ethereum_types::{Address, H256, U256};
use std::str::FromStr;
use tiny_keccak::{Hasher, Keccak};
use rlp::RlpStream;

/// Converte uma string hexadecimal para Address
pub fn hex_to_address(hex: &str) -> Option<Address> {
    let hex_str = if hex.starts_with("0x") { &hex[2..] } else { hex };
    Address::from_str(hex_str).ok()
}

/// Converte uma string hexadecimal para H256
pub fn hex_to_h256(hex: &str) -> Option<H256> {
    let hex_str = if hex.starts_with("0x") { &hex[2..] } else { hex };
    H256::from_str(hex_str).ok()
}

/// Converte uma string decimal para U256
pub fn decimal_to_u256(decimal: &str) -> Option<U256> {
    U256::from_dec_str(decimal).ok()
}

/// Formata um Address para exibição
pub fn format_address(address: &Address) -> String {
    format!("0x{:x}", address)
}

/// Formata um H256 para exibição
pub fn format_h256(hash: &H256) -> String {
    format!("0x{:x}", hash)
}

/// Formata um U256 para exibição
pub fn format_u256(value: &U256) -> String {
    value.to_string()
}

/// Verifica se um endereço é um contrato
pub async fn is_contract<P: crate::traits::RpcProvider>(provider: &P, address: &Address) -> bool {
    match provider.get_code(*address).await {
        Ok(code) => !code.is_empty(),
        Err(_) => false,
    }
}

/// Calcula o hash Keccak-256 de dados
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    let mut result = [0u8; 32];
    hasher.update(data);
    hasher.finalize(&mut result);
    result
}

/// Calcula o endereço de um contrato criado via CREATE
pub fn calculate_create_address(sender: &Address, nonce: u64) -> Address {
    // Implementação completa usando RLP encoding conforme especificação Ethereum
    let mut stream = RlpStream::new_list(2);
    stream.append(sender);
    stream.append(&nonce);
    
    let encoded = stream.out();
    let hash = keccak256(&encoded);
    
    // Os últimos 20 bytes do hash formam o endereço
    Address::from_slice(&hash[12..32])
}

/// Calcula o endereço de um contrato criado via CREATE2
pub fn calculate_create2_address(sender: &Address, salt: &H256, init_code_hash: &H256) -> Address {
    // Implementação completa do algoritmo CREATE2 conforme EIP-1014
    let mut buffer = Vec::with_capacity(1 + 20 + 32 + 32);
    
    // 0xff é um prefixo para diferenciar de outros métodos de derivação de endereço
    buffer.push(0xff);
    
    // Adiciona o endereço do criador
    buffer.extend_from_slice(sender.as_bytes());
    
    // Adiciona o salt
    buffer.extend_from_slice(salt.as_bytes());
    
    // Adiciona o hash do código de inicialização
    buffer.extend_from_slice(init_code_hash.as_bytes());
    
    // Calcula o hash Keccak-256
    let hash = keccak256(&buffer);
    
    // Os últimos 20 bytes do hash formam o endereço
    Address::from_slice(&hash[12..32])
}

/// Verifica se um endereço é um contrato ERC20
pub async fn is_erc20<P: crate::traits::RpcProvider>(provider: &P, address: &Address) -> bool {
    // Seletores de função ERC20
    let total_supply_selector = [0x18, 0x16, 0x0d, 0xdd]; // totalSupply()
    let balance_of_selector = [0x70, 0xa0, 0x82, 0x31]; // balanceOf(address)
    let transfer_selector = [0xa9, 0x05, 0x9c, 0xbb]; // transfer(address,uint256)
    
    // Verifica se é um contrato
    if !is_contract(provider, address).await {
        return false;
    }
    
    // Verifica se implementa as funções básicas do ERC20
    let mut has_total_supply = false;
    let mut has_balance_of = false;
    let mut has_transfer = false;
    
    // Verifica totalSupply
    let mut data = Vec::with_capacity(4);
    data.extend_from_slice(&total_supply_selector);
    if let Ok(_) = provider.call(*address, data).await {
        has_total_supply = true;
    }
    
    // Verifica balanceOf
    let mut data = Vec::with_capacity(36); // 4 bytes selector + 32 bytes address
    data.extend_from_slice(&balance_of_selector);
    data.extend_from_slice(&[0; 32]); // Endereço zero-padded
    if let Ok(_) = provider.call(*address, data).await {
        has_balance_of = true;
    }
    
    // Verifica transfer
    let mut data = Vec::with_capacity(68); // 4 bytes selector + 32 bytes address + 32 bytes amount
    data.extend_from_slice(&transfer_selector);
    data.extend_from_slice(&[0; 64]); // Endereço e valor zero-padded
    if let Ok(_) = provider.call(*address, data).await {
        has_transfer = true;
    }
    
    // É um ERC20 se implementar todas as funções básicas
    has_total_supply && has_balance_of && has_transfer
}

/// Formata um valor com decimais para exibição
pub fn format_token_amount(amount: &U256, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }
    
    let divisor = U256::from(10).pow(U256::from(decimals));
    let integer_part = amount / divisor;
    let fractional_part = amount % divisor;
    
    // Converte a parte fracionária para string com zeros à esquerda
    let fractional_str = fractional_part.to_string();
    let padding = decimals as usize - fractional_str.len();
    let mut padded_fractional = String::with_capacity(decimals as usize);
    for _ in 0..padding {
        padded_fractional.push('0');
    }
    padded_fractional.push_str(&fractional_str);
    
    // Remove zeros à direita
    while padded_fractional.ends_with('0') && !padded_fractional.is_empty() {
        padded_fractional.pop();
    }
    
    if padded_fractional.is_empty() {
        integer_part.to_string()
    } else {
        format!("{}.{}", integer_part, padded_fractional)
    }
}

/// Calcula o hash de uma mensagem no formato Ethereum Signed Message
pub fn eth_message_hash(message: &[u8]) -> H256 {
    // Prefixo padrão do Ethereum para assinatura de mensagens
    let prefix = "\x19Ethereum Signed Message:\n";
    let message_len = message.len().to_string();
    
    // Concatena o prefixo, o tamanho da mensagem e a mensagem
    let mut buffer = Vec::with_capacity(prefix.len() + message_len.len() + message.len());
    buffer.extend_from_slice(prefix.as_bytes());
    buffer.extend_from_slice(message_len.as_bytes());
    buffer.extend_from_slice(message);
    
    // Calcula o hash Keccak-256
    let hash = keccak256(&buffer);
    H256::from_slice(&hash)
}

/// Recupera o endereço que assinou uma mensagem
pub fn recover_signer(message_hash: &H256, signature: &[u8]) -> Option<Address> {
    if signature.len() != 65 {
        return None;
    }
    
    let r = H256::from_slice(&signature[0..32]);
    let s = H256::from_slice(&signature[32..64]);
    let v = signature[64];
    
    // Recupera a chave pública usando a biblioteca secp256k1
    let secp = secp256k1::Secp256k1::new();
    let recovery_id = secp256k1::ecdsa::RecoveryId::from_i32(v as i32 - 27).ok()?;
    let message = secp256k1::Message::from_slice(message_hash.as_bytes()).ok()?;
    
    // Cria a assinatura recuperável
    let mut sig_bytes = [0u8; 64];
    sig_bytes[0..32].copy_from_slice(&r.0);
    sig_bytes[32..64].copy_from_slice(&s.0);
    let recoverable_sig = secp256k1::ecdsa::RecoverableSignature::from_compact(&sig_bytes, recovery_id).ok()?;
    
    let public_key = secp.recover_ecdsa(&message, &recoverable_sig).ok()?;
    let public_key_serialized = public_key.serialize_uncompressed();
    
    // O endereço Ethereum é os últimos 20 bytes do hash Keccak-256 da chave pública (sem o primeiro byte)
    let hash = keccak256(&public_key_serialized[1..]);
    Some(Address::from_slice(&hash[12..32]))
}
