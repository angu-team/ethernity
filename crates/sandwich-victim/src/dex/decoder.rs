use ethers::abi::{AbiParser, Function, Token};
use serde::{Deserialize, Serialize};

/// Funções de swap suportadas em routers compatíveis com Uniswap V2
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapFunction {
    SwapExactTokensForTokens,
    SwapTokensForExactTokens,
    SwapExactETHForTokens,
    SwapTokensForExactETH,
    SwapExactTokensForETH,
    ETHForExactTokens,
    SwapExactTokensForTokensSupportingFeeOnTransferTokens,
    SwapExactETHForTokensSupportingFeeOnTransferTokens,
    SwapExactTokensForETHSupportingFeeOnTransferTokens,
}

impl SwapFunction {
    fn signature(&self) -> &'static str {
        match self {
            SwapFunction::SwapExactTokensForTokens => {
                "swapExactTokensForTokens(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::SwapTokensForExactTokens => {
                "swapTokensForExactTokens(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactETHForTokens => {
                "swapExactETHForTokens(uint256,address[],address,uint256)"
            }
            SwapFunction::SwapTokensForExactETH => {
                "swapTokensForExactETH(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactTokensForETH => {
                "swapExactTokensForETH(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::ETHForExactTokens => {
                "swapETHForExactTokens(uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactTokensForTokensSupportingFeeOnTransferTokens => {
                "swapExactTokensForTokensSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokens => {
                "swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256)"
            }
            SwapFunction::SwapExactTokensForETHSupportingFeeOnTransferTokens => {
                "swapExactTokensForETHSupportingFeeOnTransferTokens(uint256,uint256,address[],address,uint256)"
            }
        }
    }
}

/// Identifica qual função de swap foi invocada
pub fn detect_swap_function(data: &[u8]) -> Option<(SwapFunction, Function)> {
    if data.len() < 4 {
        return None;
    }
    let selector = &data[..4];
    let mut parser = AbiParser::default();
    for func in [
        SwapFunction::SwapExactTokensForTokens,
        SwapFunction::SwapTokensForExactTokens,
        SwapFunction::SwapExactETHForTokens,
        SwapFunction::SwapTokensForExactETH,
        SwapFunction::SwapExactTokensForETH,
        SwapFunction::ETHForExactTokens,
        SwapFunction::SwapExactTokensForTokensSupportingFeeOnTransferTokens,
        SwapFunction::SwapExactETHForTokensSupportingFeeOnTransferTokens,
        SwapFunction::SwapExactTokensForETHSupportingFeeOnTransferTokens,
    ] {
        let f = parser.parse_function(func.signature()).expect("abi parse");
        if selector == f.short_signature() {
            return Some((func, f));
        }
    }
    None
}

const UNIVERSAL_EXECUTE_SELECTOR: [u8; 4] = [0x35, 0x93, 0x56, 0x4c];

/// Decodifica chamadas ao Universal Router e extrai o primeiro swap
pub fn decode_universal_execute(data: &[u8]) -> Option<(SwapFunction, Vec<Token>)> {
    if data.len() < 4 || data[..4] != UNIVERSAL_EXECUTE_SELECTOR {
        return None;
    }

    let mut parser = AbiParser::default();
    let exec = parser
        .parse_function("execute(bytes,bytes[],uint256)")
        .ok()?;
    let tokens = exec.decode_input(&data[4..]).ok()?;
    let commands: Vec<u8> = tokens[0].clone().into_bytes().unwrap_or_default();
    let inputs_tokens = tokens[1].clone().into_array().unwrap_or_default();
    let inputs: Vec<Vec<u8>> = inputs_tokens
        .into_iter()
        .map(|t| t.into_bytes().unwrap_or_default())
        .collect();

    for (idx, cmd) in commands.iter().enumerate() {
        let cmd_type = cmd & 0x3f;
        match cmd_type {
            0x08 => {
                let f = parser
                    .parse_function("v2SwapExactInput(address,uint256,uint256,address[],address)")
                    .ok()?;
                let toks = f.decode_input(&inputs.get(idx)?[..]).ok()?;
                return Some((SwapFunction::SwapExactTokensForTokens, toks));
            }
            0x09 => {
                let f = parser
                    .parse_function("v2SwapExactOutput(address,uint256,uint256,address[],address)")
                    .ok()?;
                let toks = f.decode_input(&inputs.get(idx)?[..]).ok()?;
                return Some((SwapFunction::SwapTokensForExactTokens, toks));
            }
            _ => {}
        }
    }
    None
}
