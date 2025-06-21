use crate::{TokenTransfer, TokenType};
use crate::utils;
use ethereum_types::U256;

pub async fn extract_token_transfers(receipt: &serde_json::Value) -> Result<Vec<TokenTransfer>, ()> {
    let mut transfers = Vec::new();
    if let Some(logs) = receipt.get("logs").and_then(|l| l.as_array()) {
        for (log_index, log) in logs.iter().enumerate() {
            if let Some(tr) = parse_token_transfer_log(log, log_index).await? {
                transfers.push(tr);
            }
        }
    }
    Ok(transfers)
}

async fn parse_token_transfer_log(log: &serde_json::Value, call_index: usize) -> Result<Option<TokenTransfer>, ()> {
    let topics = match log.get("topics").and_then(|t| t.as_array()) {
        Some(t) if t.len() >= 3 => t,
        _ => return Ok(None),
    };
    let transfer_sig = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
    if topics[0].as_str().unwrap_or("") != transfer_sig { return Ok(None); }
    let from = utils::parse_address(topics[1].as_str().unwrap_or(""));
    let to = utils::parse_address(topics[2].as_str().unwrap_or(""));
    let (token_type, amount, token_id) = if topics.len() == 4 {
        let token_id = utils::parse_u256_hex(topics[3].as_str().unwrap_or("0"));
        (TokenType::Erc721, U256::one(), Some(token_id))
    } else if let Some(data) = log.get("data").and_then(|d| d.as_str()) {
        let amount = utils::parse_u256_hex(data);
        (TokenType::Erc20, amount, None)
    } else {
        return Ok(None);
    };
    let token_address = utils::parse_address(log.get("address").and_then(|a| a.as_str()).unwrap_or(""));
    Ok(Some(TokenTransfer { token_type, token_address, from, to, amount, token_id, call_index }))
}
